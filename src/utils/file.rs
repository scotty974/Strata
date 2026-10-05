use std::fs::File;
use std::io::{BufReader, Read};
use std::sync::Arc;
use parquet::schema::types::Type;
use parquet::file::reader::{FileReader, SerializedFileReader};


fn parquet_metadata(reader:&SerializedFileReader<File>) -> Vec<Arc<Type>>{
    let parquet_metadata = reader.metadata();
    let fields = parquet_metadata.file_metadata().schema().get_fields();
    fields.to_vec()
}


pub fn reader_metadata(path:&String) -> std::io::Result<()>{
    let file = File::open(path).expect("Couldn't open the parquet");
    let reader = SerializedFileReader::new(file).unwrap();

    let metadata = parquet_metadata(&reader);

    for (pos, col) in metadata.iter().enumerate(){
        let name = col.name();

        let p_type = col.get_physical_type();


        println!("{}, {}, {}", pos, name, &p_type);
    }

    Ok(())
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_reader(){
        let path = String::from(r"c:\Users\FabienETHEVE\Downloads\simple-users.parquet");
        let result = reader_metadata(&path);
        assert!(result.is_ok());
    }
}