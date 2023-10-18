use std::{io::{Seek, Read, Write}, ffi::CStr, fs::{File, OpenOptions}, path::Path};
use binrw::{BinRead, BinWrite, io::{SeekFrom, BufReader}, BinResult};

#[derive(Copy, Clone, PartialEq, Eq, Debug, BinRead, BinWrite)]
#[repr(u32)]
#[brw(repr = u32)]
pub enum VtPackVersion {
    Ver1 = 1,
    Ver2 = 2
}

#[derive(Clone, Debug, BinRead, BinWrite)]
#[br(little)]
pub struct VtPackStringTable {
    pub table_size: u32,
    #[br(count = table_size)]
    pub table_data: Vec<u8>
}

#[derive(Clone, Debug, BinRead, BinWrite)]
#[br(little)]
pub struct VtPackRawEntryHeader {
    pub path_name_str_table_offset: u32,
    pub path_dir_str_table_offset: u32,
    pub unk1: u32,
    pub file_size: u64,
    pub unk2: u64,
    pub file_data_abs_offset: u64,
    pub unk3: u32,
    pub unk4: u32
}

pub const INVALID_STRING_TABLE_OFFSET: u32 = u32::MAX;

pub struct VtPackProcessedEntry {
    is_file: bool,
    path: String,
    file_size: usize,
    file_data_abs_offset: u64
}

impl VtPackProcessedEntry {
    pub fn get_path(&self) -> &String {
        &self.path
    }

    pub fn is_file(&self) -> bool {
        self.is_file
    }
    
    pub fn is_dir(&self) -> bool {
        !self.is_file
    }

    pub fn get_file_size(&self) -> usize {
        self.file_size
    }
}

#[derive(Clone, Debug, BinRead, BinWrite)]
#[br(little, magic = b"vtPack")]
pub struct VtPackRawFile {
    pub version: VtPackVersion,
    pub unk1: u32,
    pub unk2: u32,

    #[br(if(version == VtPackVersion::Ver1))]
    pub unk3_v1: u32,
    #[br(if(version == VtPackVersion::Ver2))]
    pub unk3_v2: u64,
    
    #[br(if(version == VtPackVersion::Ver1))]
    pub unk4_v1: u32,
    #[br(if(version == VtPackVersion::Ver2))]
    pub unk4_v2: u64,
    
    pub entry_count: u32,

    #[br(if(version == VtPackVersion::Ver1))]
    pub str_table_abs_offset_v1: u32,
    #[br(if(version == VtPackVersion::Ver2))]
    pub str_table_abs_offset_v2: u64,

    // Ugly, but does the trick
    #[br(seek_before = SeekFrom::Start(str_table_abs_offset_v2.max(str_table_abs_offset_v1 as u64)))]
    pub str_table: VtPackStringTable,

    #[br(count = entry_count)]
    pub entries: Vec<VtPackRawEntryHeader>
}

pub struct VtPackFile {
    raw: VtPackRawFile,
    p_entries: Vec<VtPackProcessedEntry>
}

impl VtPackFile {
    fn process_entries(&mut self) {
        self.p_entries.clear();

        for entry in self.raw.entries.iter() {
            let dir_str = if entry.path_dir_str_table_offset != INVALID_STRING_TABLE_OFFSET {
                let str_ref = &self.raw.str_table.table_data[entry.path_dir_str_table_offset as usize..];
                CStr::from_bytes_until_nul(str_ref).unwrap().to_string_lossy().to_string()
            }
            else {
                String::new()
            };

            let name_str = if entry.path_name_str_table_offset != INVALID_STRING_TABLE_OFFSET {
                let str_ref = &self.raw.str_table.table_data[entry.path_name_str_table_offset as usize..];
                CStr::from_bytes_until_nul(str_ref).unwrap().to_string_lossy().to_string()
            }
            else {
                String::new()
            };

            // TODO: easier way to ensure Rust doesn't treat these raw paths as absolute (they all start with "\")
            let mut path = format!("{}\\{}", dir_str, name_str).replace("\\\\", "\\").replace("\\", std::path::MAIN_SEPARATOR_STR);
            while path.starts_with(std::path::MAIN_SEPARATOR) {
                path.remove(0);
            }

            let p_entry = VtPackProcessedEntry {
                is_file: entry.file_data_abs_offset != 0,
                path,
                file_size: entry.file_size as usize,
                file_data_abs_offset: entry.file_data_abs_offset
            };
            self.p_entries.push(p_entry);
        }
    }

    pub fn new<R: Seek + Read>(reader: &mut R) -> BinResult<Self> {
        let raw = VtPackRawFile::read(reader)?;

        let mut file = Self {
            raw,
            p_entries: Vec::new()
        };
        file.process_entries();
        Ok(file)
    }

    pub fn list_entries(&self) -> &Vec<VtPackProcessedEntry> {
        &self.p_entries
    }

    pub fn from_file(f: &File) -> BinResult<Self> {
        let mut br = BufReader::new(f);
        Self::new(&mut br)
    }

    pub fn save_entry<R: Seek + Read, P: AsRef<Path>>(&self, reader: &mut R, entry: &VtPackProcessedEntry, out_path: P) {
        let full_path = out_path.as_ref().join(entry.path.clone());

        if entry.is_file {
            let dir_path = full_path.parent().unwrap();
            let _ = std::fs::create_dir_all(dir_path);

            let mut file_data: Vec<u8> = vec![0; entry.file_size as usize];
            reader.seek(SeekFrom::Start(entry.file_data_abs_offset)).unwrap();
            reader.read(&mut file_data).unwrap();

            let mut out_file_f = OpenOptions::new().create(true).write(true).truncate(true).open(full_path).unwrap();
            out_file_f.write(&file_data).unwrap();
        }
        else {
            let _ = std::fs::create_dir_all(full_path);
        }
    }

    pub fn export_all<R: Seek + Read, P: AsRef<Path> + Clone>(&self, reader: &mut R, out_path: P) {
        let _ = std::fs::remove_dir_all(out_path.as_ref());
        let _ = std::fs::create_dir(out_path.as_ref());

        for p_entry in self.p_entries.iter() {
            self.save_entry(reader, p_entry, out_path.clone());
        }
    }
}
