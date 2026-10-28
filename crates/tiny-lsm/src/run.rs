use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use crate::{Key, RecordValue, TinyLsmError, Value};

const MAGIC: &[u8; 8] = b"TINYLSM1";
const FORMAT_VERSION: u16 = 1;
const MAX_FIELD_LEN: u64 = 1 << 30;
const MAX_RECORDS: u64 = 10_000_000;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RunId(u64);

impl RunId {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Generation(u64);

impl Generation {
    #[must_use]
    pub const fn new(generation: u64) -> Self {
        Self(generation)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug)]
pub(crate) struct SortedRun {
    pub(crate) generation: Generation,
    pub(crate) entries: Vec<(Key, RecordValue)>,
}

impl SortedRun {
    pub(crate) fn write(
        directory: &Path,
        generation: Generation,
        entries: &[(Key, RecordValue)],
    ) -> Result<Self, TinyLsmError> {
        let final_path = run_path(directory, generation);
        let temp_path = directory.join(format!(".run-{}.tmp", generation.get()));

        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|source| io_error("creating temporary run", &temp_path, source))?;

        let result = (|| {
            let mut writer = io::BufWriter::new(file);
            writer
                .write_all(MAGIC)
                .and_then(|()| writer.write_all(&FORMAT_VERSION.to_le_bytes()))
                .and_then(|()| writer.write_all(&(entries.len() as u64).to_le_bytes()))
                .map_err(|source| io_error("writing run header", &temp_path, source))?;

            for (key, value) in entries {
                write_record(&mut writer, key, value)
                    .map_err(|source| io_error("writing run record", &temp_path, source))?;
            }

            writer
                .flush()
                .map_err(|source| io_error("flushing temporary run", &temp_path, source))?;
            writer
                .get_ref()
                .sync_all()
                .map_err(|source| io_error("syncing temporary run", &temp_path, source))?;
            drop(writer);

            fs::rename(&temp_path, &final_path)
                .map_err(|source| io_error("publishing run", &final_path, source))?;

            Ok(())
        })();

        if let Err(error) = result {
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }

        Ok(Self {
            generation,
            entries: entries.to_vec(),
        })
    }

    pub(crate) fn load(path: PathBuf, generation: Generation) -> Result<Self, TinyLsmError> {
        let file = File::open(&path).map_err(|source| io_error("opening run", &path, source))?;
        let mut reader = io::BufReader::new(file);
        let mut magic = [0; 8];
        read_exact(&mut reader, &mut magic, &path, "run magic")?;
        if &magic != MAGIC {
            return Err(corrupt(&path, "invalid run magic"));
        }

        let version = read_u16(&mut reader, &path, "format version")?;
        if version != FORMAT_VERSION {
            return Err(corrupt(&path, "unsupported run format version"));
        }

        let count = read_u64(&mut reader, &path, "record count")?;
        if count > MAX_RECORDS {
            return Err(corrupt(&path, "record count exceeds format limit"));
        }

        let capacity = usize::try_from(count)
            .map_err(|_| corrupt(&path, "record count does not fit this platform"))?;
        let mut entries = Vec::with_capacity(capacity.min(4096));

        for _ in 0..count {
            let key_len = read_u64(&mut reader, &path, "key length")?;
            let value_tag = read_u8(&mut reader, &path, "record tag")?;
            let value_len = if value_tag == 1 {
                read_u64(&mut reader, &path, "value length")?
            } else if value_tag == 0 {
                0
            } else {
                return Err(corrupt(&path, "unknown record tag"));
            };

            if key_len > MAX_FIELD_LEN || value_len > MAX_FIELD_LEN {
                return Err(corrupt(&path, "record field exceeds format limit"));
            }

            let key_len = usize::try_from(key_len)
                .map_err(|_| corrupt(&path, "key length does not fit this platform"))?;
            let value_len = usize::try_from(value_len)
                .map_err(|_| corrupt(&path, "value length does not fit this platform"))?;

            let mut key = vec![0; key_len];
            read_exact(&mut reader, &mut key, &path, "key bytes")?;

            let value = if value_tag == 0 {
                RecordValue::Tombstone
            } else {
                let mut bytes = vec![0; value_len];
                read_exact(&mut reader, &mut bytes, &path, "value bytes")?;
                RecordValue::Value(Value::from(bytes))
            };

            let key = Key::from(key);
            if entries
                .last()
                .is_some_and(|(previous, _): &(Key, RecordValue)| previous >= &key)
            {
                return Err(corrupt(&path, "records are not strictly sorted"));
            }
            entries.push((key, value));
        }

        let mut trailing = [0; 1];
        match reader.read(&mut trailing) {
            Ok(0) => {}
            Ok(_) => return Err(corrupt(&path, "trailing bytes after final record")),
            Err(source) => return Err(io_error("reading run trailer", &path, source)),
        }

        Ok(Self {
            generation,
            entries,
        })
    }
}

pub(crate) fn discover(directory: &Path) -> Result<Vec<SortedRun>, TinyLsmError> {
    remove_stale_temporary_runs(directory)?;
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)
        .map_err(|source| io_error("listing store directory", directory, source))?
    {
        let entry =
            entry.map_err(|source| io_error("reading directory entry", directory, source))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if name.starts_with("run-") && name.ends_with(".tlsm") {
            let number = name
                .strip_prefix("run-")
                .and_then(|name| name.strip_suffix(".tlsm"))
                .and_then(|number| number.parse::<u64>().ok())
                .ok_or_else(|| corrupt(&path, "invalid run filename"))?;
            if name != format!("run-{number}.tlsm") {
                return Err(corrupt(&path, "non-canonical run filename"));
            }
            paths.push((Generation::new(number), path));
        }
    }

    paths.sort_by_key(|(generation, _)| *generation);
    let mut runs = Vec::with_capacity(paths.len());
    for (generation, path) in paths {
        runs.push(SortedRun::load(path, generation)?);
    }
    Ok(runs)
}

fn remove_stale_temporary_runs(directory: &Path) -> Result<(), TinyLsmError> {
    let mut removed = false;
    for entry in fs::read_dir(directory)
        .map_err(|source| io_error("listing store directory", directory, source))?
    {
        let entry =
            entry.map_err(|source| io_error("reading directory entry", directory, source))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let is_temporary_run = name
            .strip_prefix(".run-")
            .and_then(|name| name.strip_suffix(".tmp"))
            .is_some_and(|number| {
                !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
            });
        let is_file = entry
            .file_type()
            .map_err(|source| io_error("reading temporary run metadata", &path, source))?
            .is_file();
        if is_temporary_run && is_file {
            fs::remove_file(&path)
                .map_err(|source| io_error("removing stale temporary run", &path, source))?;
            removed = true;
        }
    }
    if removed {
        sync_directory(directory)?;
    }
    Ok(())
}

fn sync_directory(directory: &Path) -> Result<(), TinyLsmError> {
    #[cfg(unix)]
    {
        File::open(directory)
            .and_then(|file| file.sync_all())
            .map_err(|source| io_error("syncing store directory", directory, source))?;
    }
    #[cfg(not(unix))]
    let _ = directory;
    Ok(())
}

pub(crate) fn next_generation(runs: &[SortedRun]) -> Result<Generation, TinyLsmError> {
    let next = runs
        .iter()
        .map(|run| run.generation.get())
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(TinyLsmError::IdentifierExhausted("run generation"))?;
    Ok(Generation::new(next))
}

pub(crate) fn run_path(directory: &Path, generation: Generation) -> PathBuf {
    directory.join(format!("run-{}.tlsm", generation.get()))
}

fn write_record(writer: &mut impl Write, key: &Key, value: &RecordValue) -> io::Result<()> {
    writer.write_all(&(key.as_bytes().len() as u64).to_le_bytes())?;
    match value {
        RecordValue::Tombstone => writer.write_all(&[0])?,
        RecordValue::Value(value) => {
            writer.write_all(&[1])?;
            writer.write_all(&(value.as_bytes().len() as u64).to_le_bytes())?;
        }
    }
    writer.write_all(key.as_bytes())?;
    if let RecordValue::Value(value) = value {
        writer.write_all(value.as_bytes())?;
    }
    Ok(())
}

fn read_exact(
    reader: &mut impl Read,
    bytes: &mut [u8],
    path: &Path,
    field: &'static str,
) -> Result<(), TinyLsmError> {
    match reader.read_exact(bytes) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::UnexpectedEof => {
            Err(corrupt(path, &format!("truncated {field}")))
        }
        Err(source) => Err(io_error(field, path, source)),
    }
}

fn read_u8(reader: &mut impl Read, path: &Path, field: &'static str) -> Result<u8, TinyLsmError> {
    let mut bytes = [0; 1];
    read_exact(reader, &mut bytes, path, field)?;
    Ok(bytes[0])
}

fn read_u16(reader: &mut impl Read, path: &Path, field: &'static str) -> Result<u16, TinyLsmError> {
    let mut bytes = [0; 2];
    read_exact(reader, &mut bytes, path, field)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u64(reader: &mut impl Read, path: &Path, field: &'static str) -> Result<u64, TinyLsmError> {
    let mut bytes = [0; 8];
    read_exact(reader, &mut bytes, path, field)?;
    Ok(u64::from_le_bytes(bytes))
}

fn io_error(operation: &'static str, path: &Path, source: io::Error) -> TinyLsmError {
    TinyLsmError::Io {
        operation,
        path: Some(path.to_path_buf()),
        source,
    }
}

fn corrupt(path: &Path, reason: &str) -> TinyLsmError {
    TinyLsmError::CorruptData {
        path: Some(path.to_path_buf()),
        reason: reason.to_owned(),
    }
}
