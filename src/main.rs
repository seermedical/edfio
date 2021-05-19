#[macro_use]
extern crate serde_derive;

mod edf_reader;

use edf_reader::{sync_reader, file_reader};

fn main() {
  let filename = "src/test_file.edf";
  let reader = file_reader::FileReader {
    filename: String::from(filename)
  };
  let edf_reader = sync_reader::SyncEDFReader::init_with_file_reader(reader).unwrap();
  let result = edf_reader.read_data_window(0, 6000).unwrap();
  println!("{:?}", result)
}
