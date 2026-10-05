mod utils;

fn main() {

    let path = String::from(r"c:\Users\FabienETHEVE\Downloads\simple-users.parquet");

    let contents = utils::file::reader_metadata(&path);

    println!("{:?}", contents);

}
