mod utils;

fn main() {

    let path = String::from(r"c:\Users\FabienETHEVE\Downloads\titanic.parquet");

    let metadata = utils::file::reader_metadata(&path);
    let contents = utils::file::read_rows(&path, 5);

    println!("{:?}", metadata);
    println!("{:?}", contents);

}
