//! Read the Vegas asset from the existing ESP-IDF wear-levelled FAT16 volume.
//! The flash callback only supports reads. An interrupted wear-level update
//! is left to the ESP-IDF importer to recover.

const SECTOR: usize = 4096;
const STATE_HEADER: usize = 64;
const STATE_RECORD: usize = 16;
const DIRECTORY_ENTRY: usize = 32;
const SCENIC_DIRECTORY: &[u8; 11] = b"SCENIC     ";
const VEGAS_FILE: &[u8; 11] = b"VEGAS   PAK";
const VEGAS_BACKUP: &[u8; 11] = b"VEGAS   BAK";
const LOS_ANGELES_FILE: &[u8; 11] = b"LOSANGELPAK";
const LOS_ANGELES_BACKUP: &[u8; 11] = b"LOSANGELBAK";
#[cfg(feature = "native-southwest-file")]
const SOUTHWEST_FILE: &[u8; 11] = b"SOUTHWSTPAK";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageError {
    Read,
    WearLevelState,
    FatVolume,
    FatChain,
    MissingFile,
    FileSize,
}

type Result<T> = core::result::Result<T, StorageError>;

pub struct VegasVolume<F> {
    flash: F,
    sectors: usize,
    dummy: usize,
    moves: usize,
    fat: Fat16,
}

#[derive(Clone, Copy)]
struct Fat16 {
    fat_offset: usize,
    root_offset: usize,
    root_sectors: usize,
    data_offset: usize,
    sectors_per_cluster: usize,
    clusters: usize,
}

#[derive(Clone, Copy)]
pub struct VegasFile {
    first_cluster: usize,
    pub len: usize,
}

pub struct FileIndex<'a> {
    clusters: &'a [u16],
    length: usize,
}

impl<F: FnMut(usize, &mut [u8]) -> bool> VegasVolume<F> {
    pub fn open(mut flash: F, partition_bytes: usize, scratch: &mut [u8; SECTOR]) -> Result<Self> {
        if partition_bytes % SECTOR != 0 || partition_bytes < 6 * SECTOR {
            return Err(StorageError::WearLevelState);
        }
        let raw_sectors = partition_bytes / SECTOR;
        let cfg_offset = partition_bytes - SECTOR;
        read(&mut flash, cfg_offset, &mut scratch[..36])?;
        if u32_at(scratch, 0) != 0
            || u32_at(scratch, 4) as usize != partition_bytes
            || u32_at(scratch, 8) as usize != SECTOR
            || u32_at(scratch, 12) as usize != SECTOR
            || u32_at(scratch, 20) as usize != STATE_RECORD
            || u32_at(scratch, 24) != 2
            || wl_crc(&scratch[..32]) != u32_at(scratch, 32)
        {
            return Err(StorageError::WearLevelState);
        }

        let state_sectors = (STATE_HEADER + raw_sectors * STATE_RECORD).div_ceil(SECTOR);
        let sectors = raw_sectors
            .checked_sub(2 * state_sectors + 2)
            .filter(|&n| n > 0)
            .ok_or(StorageError::WearLevelState)?;
        let primary = cfg_offset - 2 * state_sectors * SECTOR;
        let secondary = cfg_offset - state_sectors * SECTOR;
        let read_state = |flash: &mut F, offset| -> Result<(usize, usize)> {
            read(flash, offset, &mut scratch[..STATE_HEADER])?;
            let start = u32_at(scratch, 0) as usize;
            let moves = u32_at(scratch, 8) as usize;
            let id = u32_at(scratch, 28);
            if start >= sectors + 1
                || moves >= sectors
                || u32_at(scratch, 4) as usize != sectors + 1
                || u32_at(scratch, 20) as usize != SECTOR
                || u32_at(scratch, 24) != 2
                || wl_crc(&scratch[..60]) != u32_at(scratch, 60)
            {
                return Err(StorageError::WearLevelState);
            }
            let last = STATE_HEADER + sectors * STATE_RECORD;
            let mut cursor = STATE_HEADER;
            let mut dummy = 0;
            while cursor <= last {
                let bytes = (last + STATE_RECORD - cursor).min(SECTOR);
                read(flash, offset + cursor, &mut scratch[..bytes])?;
                for record in scratch[..bytes].chunks_exact(STATE_RECORD) {
                    if record.iter().all(|&value| value == u8::MAX) {
                        return Ok((dummy, moves));
                    }
                    for word in 0..4 {
                        let value = id.wrapping_add((dummy * 4 + word) as u32);
                        if wl_crc(&value.to_le_bytes()) != u32_at(record, word * 4) {
                            return Err(StorageError::WearLevelState);
                        }
                    }
                    dummy += 1;
                }
                cursor += bytes;
            }
            Err(StorageError::WearLevelState)
        };
        let mut read_state = read_state;
        let (dummy, moves) = read_state(&mut flash, primary)?;
        if read_state(&mut flash, secondary)? != (dummy, moves) {
            return Err(StorageError::WearLevelState);
        }
        let physical_boot = address(sectors, dummy, moves, 0);
        read(&mut flash, physical_boot, &mut scratch[..512])?;
        let fat = Fat16::parse(scratch, sectors)?;
        Ok(Self {
            flash,
            sectors,
            dummy,
            moves,
            fat,
        })
    }

    pub fn vegas_file(&mut self, scratch: &mut [u8; SECTOR]) -> Result<VegasFile> {
        self.scenic_file(VEGAS_FILE, VEGAS_BACKUP, scratch)
    }

    pub fn los_angeles_file(&mut self, scratch: &mut [u8; SECTOR]) -> Result<VegasFile> {
        self.scenic_file(LOS_ANGELES_FILE, LOS_ANGELES_BACKUP, scratch)
    }

    #[cfg(feature = "native-southwest-file")]
    pub fn southwest_file(&mut self, scratch: &mut [u8; SECTOR]) -> Result<VegasFile> {
        self.scenic_file(SOUTHWEST_FILE, b"SOUTHWSTBAK", scratch)
    }

    fn scenic_file(
        &mut self, name: &[u8; 11], backup: &[u8; 11], scratch: &mut [u8; SECTOR],
    ) -> Result<VegasFile> {
        let directory = self.find_directory_entries(
            self.fat.root_offset,
            self.fat.root_sectors,
            |entry| &entry[..11] == SCENIC_DIRECTORY && entry[11] & 0x10 != 0,
            scratch,
        )?;
        let first_cluster = u16_at(&directory, 26) as usize;
        let file = self
            .find_in_cluster_chain(first_cluster, name, scratch)
            .or_else(|error| {
                if error != StorageError::MissingFile {
                    return Err(error);
                }
                self.find_in_cluster_chain(first_cluster, backup, scratch)
            })?;
        Ok(VegasFile {
            first_cluster: u16_at(&file, 26) as usize,
            len: u32_at(&file, 28) as usize,
        })
    }

    pub fn index_len(&self, file: VegasFile) -> usize {
        file.len.div_ceil(self.fat.sectors_per_cluster * SECTOR)
    }

    pub fn index_file<'a>(&mut self, file: VegasFile, slots: &'a mut [u16]) -> Result<FileIndex<'a>> {
        if slots.is_empty() || slots.len() != self.index_len(file) || slots.len() > self.fat.clusters {
            return Err(StorageError::FileSize);
        }
        let mut next = Some(file.first_cluster);
        for slot in slots.iter_mut() {
            let cluster = next.ok_or(StorageError::FatChain)?;
            self.fat.cluster_offset(cluster)?;
            *slot = cluster as u16;
            next = self.next_cluster(cluster)?;
        }
        if next.is_some() {
            return Err(StorageError::FatChain);
        }
        Ok(FileIndex { clusters: slots, length: file.len })
    }

    pub fn read_at(&mut self, file: &FileIndex<'_>, mut offset: usize, mut out: &mut [u8]) -> Result<()> {
        if offset.checked_add(out.len()).is_none_or(|end| end > file.length) {
            return Err(StorageError::FileSize);
        }
        let cluster_bytes = self.fat.sectors_per_cluster * SECTOR;
        while !out.is_empty() {
            let cluster = file.clusters[offset / cluster_bytes] as usize;
            let logical = self.fat.cluster_offset(cluster)? + offset % cluster_bytes;
            let count = out.len().min(SECTOR - logical % SECTOR);
            let (target, rest) = out.split_at_mut(count);
            self.read_logical(logical, target)?;
            offset += count;
            out = rest;
        }
        Ok(())
    }

    pub fn read_file(
        &mut self,
        file: VegasFile,
        mut output: &mut [u8],
        scratch: &mut [u8; SECTOR],
    ) -> Result<()> {
        if output.len() != file.len || output.is_empty() {
            return Err(StorageError::FileSize);
        }
        let mut cluster = file.first_cluster;
        while !output.is_empty() {
            let start = self.fat.cluster_offset(cluster)?;
            for sector in 0..self.fat.sectors_per_cluster {
                let count = output.len().min(SECTOR);
                self.read_logical(start + sector * SECTOR, &mut scratch[..count])?;
                let (target, rest) = output.split_at_mut(count);
                target.copy_from_slice(&scratch[..count]);
                output = rest;
                if output.is_empty() {
                    break;
                }
            }
            let next = self.next_cluster(cluster)?;
            match (output.is_empty(), next) {
                (true, None) => return Ok(()),
                (false, Some(next)) => cluster = next,
                _ => return Err(StorageError::FatChain),
            }
        }
        Err(StorageError::FatChain)
    }

    fn find_in_cluster_chain(
        &mut self,
        mut cluster: usize,
        name: &[u8; 11],
        scratch: &mut [u8; SECTOR],
    ) -> Result<[u8; DIRECTORY_ENTRY]> {
        for _ in 0..self.fat.clusters {
            let start = self.fat.cluster_offset(cluster)?;
            match self.find_directory_entries(
                start,
                self.fat.sectors_per_cluster,
                |entry| &entry[..11] == name && entry[11] & 0x10 == 0,
                scratch,
            ) {
                Ok(entry) => return Ok(entry),
                Err(StorageError::MissingFile) => {}
                Err(error) => return Err(error),
            }
            match self.next_cluster(cluster)? {
                Some(next) => cluster = next,
                None => return Err(StorageError::MissingFile),
            }
        }
        Err(StorageError::FatChain)
    }

    fn find_directory_entries(
        &mut self,
        start: usize,
        sectors: usize,
        matches: impl Fn(&[u8]) -> bool,
        scratch: &mut [u8; SECTOR],
    ) -> Result<[u8; DIRECTORY_ENTRY]> {
        for sector in 0..sectors {
            self.read_logical(start + sector * SECTOR, scratch)?;
            for entry in scratch.chunks_exact(DIRECTORY_ENTRY) {
                if entry[0] == 0 {
                    return Err(StorageError::MissingFile);
                }
                if entry[0] != 0xe5 && entry[11] & 0x08 == 0 && matches(entry) {
                    return Ok(entry.try_into().unwrap());
                }
            }
        }
        Err(StorageError::MissingFile)
    }

    fn next_cluster(&mut self, cluster: usize) -> Result<Option<usize>> {
        let mut bytes = [0; 2];
        self.read_logical(self.fat.fat_offset + cluster * 2, &mut bytes)?;
        let next = u16::from_le_bytes(bytes) as usize;
        if next >= 0xfff8 {
            return Ok(None);
        }
        self.fat.cluster_offset(next).map(|_| Some(next))
    }

    fn read_logical(&mut self, offset: usize, target: &mut [u8]) -> Result<()> {
        if offset >= self.sectors * SECTOR || target.len() > SECTOR - offset % SECTOR {
            return Err(StorageError::FatVolume);
        }
        read(
            &mut self.flash,
            address(self.sectors, self.dummy, self.moves, offset),
            target,
        )
    }
}

impl Fat16 {
    fn parse(boot: &[u8], volume_sectors: usize) -> Result<Self> {
        let sectors_per_cluster = usize::from(boot[13]);
        let reserved = u16_at(boot, 14) as usize;
        let tables = usize::from(boot[16]);
        let root_sectors = (u16_at(boot, 17) as usize * DIRECTORY_ENTRY).div_ceil(SECTOR);
        let fat_sectors = u16_at(boot, 22) as usize;
        let short_total = u16_at(boot, 19) as usize;
        let sectors = if short_total != 0 {
            short_total
        } else {
            u32_at(boot, 32) as usize
        };
        if boot[510..512] != [0x55, 0xaa]
            || u16_at(boot, 11) as usize != SECTOR
            || !sectors_per_cluster.is_power_of_two()
            || reserved == 0
            || tables == 0
            || fat_sectors == 0
            || root_sectors == 0
            || sectors > volume_sectors
        {
            return Err(StorageError::FatVolume);
        }
        let root = reserved + tables * fat_sectors;
        let data = root + root_sectors;
        let clusters =
            sectors.checked_sub(data).ok_or(StorageError::FatVolume)? / sectors_per_cluster;
        if !(4_085..65_525).contains(&clusters) || (clusters + 2) * 2 > fat_sectors * SECTOR {
            return Err(StorageError::FatVolume);
        }
        Ok(Self {
            fat_offset: reserved * SECTOR,
            root_offset: root * SECTOR,
            root_sectors,
            data_offset: data * SECTOR,
            sectors_per_cluster,
            clusters,
        })
    }

    fn cluster_offset(self, cluster: usize) -> Result<usize> {
        if !(2..self.clusters + 2).contains(&cluster) {
            return Err(StorageError::FatChain);
        }
        Ok(self.data_offset + (cluster - 2) * self.sectors_per_cluster * SECTOR)
    }
}

fn address(sectors: usize, dummy: usize, moves: usize, logical: usize) -> usize {
    let logical_sector = logical / SECTOR;
    let rotated = (sectors + logical_sector - moves) % sectors;
    (rotated + usize::from(rotated >= dummy)) * SECTOR + logical % SECTOR
}

fn read(
    flash: &mut impl FnMut(usize, &mut [u8]) -> bool,
    offset: usize,
    target: &mut [u8],
) -> Result<()> {
    if flash(offset, target) {
        Ok(())
    } else {
        Err(StorageError::Read)
    }
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn wl_crc(bytes: &[u8]) -> u32 {
    // CRC-32/ISO-HDLC with the ESP-IDF wear-levelling seed (0xffff_ffff).
    let mut crc = 0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}
