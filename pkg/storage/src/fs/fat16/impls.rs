use super::*;
use alloc::string::ToString;

impl Fat16Impl {
    pub fn new(inner: impl BlockDevice<Block512>) -> Self {
        let mut block = Block::default();
        let block_size = Block512::size();

        inner.read_block(0, &mut block).unwrap();
        let bpb = Fat16Bpb::new(block.as_ref()).unwrap();

        trace!("Loading Fat16 Volume: {:#?}", bpb);

        // HINT: FirstDataSector = BPB_ResvdSecCnt + (BPB_NumFATs * FATSz) + RootDirSectors;
        let fat_start = bpb.reserved_sector_count() as usize;
        let root_dir_size = {
            let root_entries_count = bpb.root_entries_count() as usize;
            (root_entries_count * DirEntry::LEN + block_size - 1) / block_size
        };
        let first_root_dir_sector = {
            let fat_count = bpb.fat_count() as usize;
            let sectors_per_fat = bpb.sectors_per_fat() as usize;

            fat_start + (fat_count * sectors_per_fat)
        };
        let first_data_sector = first_root_dir_sector + root_dir_size;

        trace!(
            "BPB: Resv={}  NumFATs={}  FATSz16={}  BytsPerSec={}  SecPerClus={}",
            bpb.reserved_sector_count(),
            bpb.fat_count(),
            bpb.sectors_per_fat(),
            bpb.bytes_per_sector(),
            bpb.sectors_per_cluster()
        );

        trace!(
            "first_root_dir_sector={}  first_data_sector={}",
            first_root_dir_sector, first_data_sector
        );

        Self {
            bpb,
            inner: Box::new(inner),
            fat_start,
            first_data_sector,
            first_root_dir_sector,
        }
    }

    pub fn cluster_to_sector(&self, cluster: &Cluster) -> usize {
        match *cluster {
            Cluster::ROOT_DIR => self.first_root_dir_sector,
            Cluster(c) => {
                // FIXME: calculate the first sector of the cluster
                // HINT: FirstSectorofCluster = ((N – 2) * BPB_SecPerClus) + FirstDataSector;
                let n = c as usize - 2;
                let first_sector_of_cluster =
                    (n * self.bpb.sectors_per_cluster() as usize) + self.first_data_sector;
                return first_sector_of_cluster;
            }
        }
    }

    // FIXME: YOU NEED TO IMPLEMENT THE FILE SYSTEM OPERATIONS HERE
    //      - read the FAT and get next cluster
    //      - traverse the cluster chain and read the data
    //      - parse the path
    //      - open the root directory
    //      - ...
    //      - finally, implement the FileSystem trait for Fat16 with `self.handle`
    pub fn next_cluster(&self, cluster: Cluster) -> Result<Cluster> {
        if cluster == Cluster::ROOT_DIR {
            return Ok(Cluster::ROOT_DIR);
        }

        let fat_offset = (cluster.0 as usize) * 2;
        let sector_index = self.fat_start + (fat_offset / BLOCK_SIZE);
        let within = fat_offset % BLOCK_SIZE;

        let mut block = Block::default();
        self.inner.read_block(sector_index, &mut block)?;

        let mut entry_bytes = [0u8; 2];
        entry_bytes[0] = block[within];
        entry_bytes[1] = if within == BLOCK_SIZE - 1 {
            block = Block::default();
            self.inner.read_block(sector_index + 1, &mut block)?;
            block[0]
        } else {
            block[within + 1]
        };

        let raw = u16::from_le_bytes(entry_bytes);
        match raw {
            0xFFF8..=0xFFFF => Ok(Cluster::END_OF_FILE),
            0xFFF7 => Err(FsError::BadCluster),
            _ => Ok(Cluster(raw as u32)),
        }
    }

    pub fn name_to_entry(&self, dir: &Directory, name: &str) -> Result<DirEntry> {
        let short_name = ShortFileName::parse(name)?;

        let sectors_per_cluster = self.bpb.sectors_per_cluster() as usize;
        let root_dir_sectors = self.first_data_sector - self.first_root_dir_sector;

        let mut cluster = dir.cluster;
        let mut block = Block::default();

        loop {
            let first_sector = self.cluster_to_sector(&cluster);
            let sector_cnt = if cluster == Cluster::ROOT_DIR {
                root_dir_sectors
            } else {
                sectors_per_cluster
            };

            for sec in first_sector..first_sector + sector_cnt {
                self.inner.read_block(sec, &mut block)?;

                for i in 0..BLOCK_SIZE / DirEntry::LEN {
                    let off = i * DirEntry::LEN;
                    let end = off + DirEntry::LEN;
                    let ent = DirEntry::parse(&block[off..end])?;

                    if !ent.is_valid() || ent.is_long_name() {
                        continue;
                    }
                    if ent.filename.matches(&short_name) {
                        return Ok(ent);
                    }
                }
            }

            if cluster == Cluster::ROOT_DIR {
                return Err(FsError::FileNotFound);
            }
            cluster = match self.next_cluster(cluster) {
                Ok(Cluster::END_OF_FILE) | Err(FsError::EndOfFile) => {
                    return Err(FsError::FileNotFound);
                }
                Ok(next) => next,
                Err(e) => return Err(e),
            }
        }
    }

    pub fn parse_path(&self, root_path: &str) -> Option<Directory> {
        let mut cur_dir = self.root_dir().ok()?;

        let comps: Vec<&str> = root_path
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        if comps.is_empty() {
            return Some(cur_dir);
        }

        for comp in comps {
            let entry = match self.name_to_entry(&cur_dir, comp) {
                Ok(e) => e,
                Err(e) => {
                    warn!("File not found: {}, {:?}", root_path, e);
                    return None;
                }
            };

            if !entry.is_directory() {
                warn!("Not a directory: {}", comp);
                return None;
            }

            cur_dir = Directory::from_entry(entry);
        }

        Some(cur_dir)
    }

    pub fn root_dir(&self) -> Result<Directory> {
        Ok(Directory::new(Cluster::ROOT_DIR))
    }
}

impl FileSystem for Fat16 {
    fn read_dir(&self, path: &str) -> Result<Box<dyn Iterator<Item = Metadata> + Send>> {
        // FIXME: read dir and return an iterator for all entries
        // info!("Reading directory: {}", path);
        let dir = self.handle.parse_path(path).ok_or(FsError::NotADirectory)?;

        let mut metas = Vec::<Metadata>::new();

        let sectors_per_cluster = self.handle.bpb.sectors_per_cluster() as usize;
        let root_dir_sectors = self.handle.first_data_sector - self.handle.first_root_dir_sector;
        let mut cluster = dir.cluster;
        let mut block = Block::default();

        loop {
            let first_sector = self.handle.cluster_to_sector(&cluster);
            let sector_cnt = if cluster == Cluster::ROOT_DIR {
                root_dir_sectors
            } else {
                sectors_per_cluster
            };

            // info!(
            //     "Reading directory: {}, cluster: {}, first sector: {}, sectors: {}",
            //     path, cluster.0, first_sector, sector_cnt
            // );

            for sec in first_sector..first_sector + sector_cnt {
                self.handle.inner.read_block(sec, &mut block)?;

                for i in 0..BLOCK_SIZE / DirEntry::LEN {
                    let off = i * DirEntry::LEN;
                    let end = off + DirEntry::LEN;
                    let ent = DirEntry::parse(&block[off..end])?;

                    trace!(
                        "attr={:02X}, raw[0]={:02X}",
                        ent.attributes.bits(),
                        ent.filename.name[0]
                    );

                    if !ent.is_valid() || ent.is_long_name() {
                        continue;
                    }

                    let metadata: Metadata = (&ent).try_into().unwrap();
                    metas.push(metadata);
                }
            }

            if cluster == Cluster::ROOT_DIR {
                break;
            }

            cluster = match self.handle.next_cluster(cluster) {
                Ok(Cluster::END_OF_FILE) | Err(FsError::EndOfFile) => break,
                Ok(next) => next,
                Err(e) => return Err(e),
            };
        }

        // info!("Read directory: {}, entries: {}", path, metas.len());

        Ok(Box::new(metas.into_iter()))
    }

    fn open_file(&self, path: &str) -> Result<FileHandle> {
        // FIXME: open file and return a file handle
        let (parent_path, filename) = match path.rfind('/') {
            None => ("", path),
            Some(idx) if idx == path.len() - 1 => {
                return Err(FsError::NotAFile);
            }
            Some(idx) => (&path[..idx], &path[idx + 1..]),
        };

        let parent_dir = self
            .handle
            .parse_path(parent_path)
            .ok_or(FsError::FileNotFound)?;

        let entry = self.handle.name_to_entry(&parent_dir, filename)?;
        if entry.is_directory() {
            return Err(FsError::NotAFile);
        }

        let file = File::new(self.handle.clone(), entry.clone());
        let metadata = (&entry).try_into().unwrap();
        Ok(FileHandle::new(metadata, Box::new(file)))
    }

    fn metadata(&self, path: &str) -> Result<Metadata> {
        // FIXME: read metadata of the file / dir
        if path.trim_matches('/').is_empty() {
            return Ok(Metadata::root());
        }

        let (parent_path, filename) = match path.rfind('/') {
            None => ("", path),
            Some(idx) if idx == path.len() - 1 => {
                return Err(FsError::NotAFile);
            }
            Some(idx) => (&path[..idx], &path[idx + 1..]),
        };

        let dir = match self.handle.parse_path(parent_path) {
            Some(d) => d,
            None => return Err(FsError::FileNotFound),
        };

        let entry = match self.handle.name_to_entry(&dir, filename) {
            Ok(e) => e,
            Err(FsError::FileNotFound) => return Err(FsError::FileNotFound),
            Err(e) => return Err(e),
        };
        let metadata: Metadata = (&entry).try_into().unwrap();
        Ok(metadata)
    }

    fn exists(&self, path: &str) -> Result<bool> {
        // FIXME: check if the file / dir exists
        if path.trim_matches('/').is_empty() {
            return Ok(true);
        }

        let (parent_path, filename) = match path.rfind('/') {
            None => ("", path),
            Some(idx) if idx == path.len() - 1 => {
                return Err(FsError::NotAFile);
            }
            Some(idx) => (&path[..idx], &path[idx + 1..]),
        };

        let dir = match self.handle.parse_path(parent_path) {
            Some(d) => d,
            None => return Ok(false),
        };

        match self.handle.name_to_entry(&dir, filename) {
            Ok(_) => Ok(true),
            Err(FsError::FileNotFound) => return Ok(false),
            Err(e) => return Err(e),
        }
    }
}
