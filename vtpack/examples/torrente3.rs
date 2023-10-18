use std::fs::File;
use std::io::BufReader;
use vtpack::VtPackFile;

fn main() {
    let vtpack_file = File::open("torrent3.vpk").unwrap();
    let mut vtpack_reader = BufReader::new(&vtpack_file);
    let vtpack = VtPackFile::new(&mut vtpack_reader).unwrap();

    for entry in vtpack.list_entries() {
        println!("> {} (file: {:?})", entry.get_path(), entry.is_file());
    }

    vtpack.export_all(&mut vtpack_reader, "tor3_vpk_out");
}