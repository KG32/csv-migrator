use clap::Parser;
use csv::StringRecord;
use std::{
    error::Error,
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

#[derive(Parser, Debug, Clone, clap::ValueEnum)]
pub enum Mode {
    Insert,
}

#[derive(Parser, Debug, Clone)]
pub struct Config {
    pub mode: Mode,
    /// path to dir with csv files
    pub path: String,
    /// column name
    pub column: String,
    /// default value for new column
    pub default_value: String,
    /// the position where the column is inserted from the left (starting with 1)
    pub order: i32,
}

fn main() {
    let config = Config::parse();
    run(config).unwrap_or_else(|_| println!("Migration failed"));
}

fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let Config {
        mode,
        path,
        column,
        default_value,
        order,
    } = config;

    let files = get_csv_files(&path)?;

    match mode {
        Mode::Insert => {
            println!(
                "Inserting {} with default value {} @ {}",
                column, default_value, order
            );
            for file in files {
                println!("\nMigrating {:?}", file.canonicalize().unwrap());
                insert_column(&file, &column, &default_value, order)?;
            }
        }
    }

    Ok(())
}

fn insert_column(
    path: &PathBuf,
    column: &str,
    default_value: &str,
    order: i32,
) -> Result<(), Box<dyn Error>> {
    let mut content = String::new();
    File::open(path)?.read_to_string(&mut content)?;
    let mut reader = csv::Reader::from_reader(content.as_bytes());
    let mut writer = csv::Writer::from_path(path)?;

    // set headers
    let headers = reader.headers()?.clone();
    let mut new_headers = StringRecord::new();
    for (i, header) in headers.iter().enumerate() {
        if i as i32 == (order - 1) {
            new_headers.push_field(column);
        }
        new_headers.push_field(header);
    }
    writer.write_record(&new_headers)?;

    // set values
    for record in reader.records() {
        let record = record?;
        let mut new_record = StringRecord::new();
        for (j, field) in record.iter().enumerate() {
            if j as i32 == (order - 1) {
                new_record.push_field(default_value);
            }
            new_record.push_field(field);
        }
        writer.write_record(&new_record)?;
    }

    Ok(())
}

fn get_csv_files(path: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut csv_file_paths: Vec<PathBuf> = vec![];
    let entries = fs::read_dir(path)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let mut traversal_res = get_csv_files(path.to_str().unwrap())?;
            csv_file_paths.append(&mut traversal_res);
        }
        let extension = path.extension().unwrap_or_default();
        if extension.to_ascii_lowercase() == "csv" {
            csv_file_paths.push(path);
        }
    }
    Ok(csv_file_paths)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_insert_column() {
        fs::create_dir_all("test_files").unwrap();
        let mut path = PathBuf::new();
        path.push("test_files/test.csv");
        let mut file = File::create(path.clone()).unwrap();
        file.write_all(
            b"H1,H2,H3,H4,H5,H6,H7,H8,H9\nV1,V2,V3,V4,V5,V6,V7,V8,V9\nV11,V22,V33,V44,V55,V66,V77,V88,V99",
        ).unwrap();

        let config = Config {
            mode: Mode::Insert,
            path: "test_files".to_string(),
            column: "H_new".to_string(),
            default_value: "V_new".to_string(),
            order: 3,
        };
        run(config).unwrap();
        let mut modified_file = File::open(path.clone()).unwrap();
        let mut modified_content = String::new();
        modified_file.read_to_string(&mut modified_content).unwrap();
        assert_eq!(
            modified_content,
            String::from(
                "H1,H2,H_new,H3,H4,H5,H6,H7,H8,H9\nV1,V2,V_new,V3,V4,V5,V6,V7,V8,V9\nV11,V22,V_new,V33,V44,V55,V66,V77,V88,V99\n"
            )
        )
    }
}
