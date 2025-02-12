use clap::{arg, command, Args, Parser, Subcommand};
use colored::Colorize;
use csv::StringRecord;
use std::{
    error::Error,
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

trait Migration {
    type ConfigType;
    fn new(config: Self::ConfigType) -> Self;
    fn run(&self) -> Result<(), Box<dyn Error>>;
    fn get_csv_files(&self, path: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        let mut csv_file_paths: Vec<PathBuf> = vec![];
        let entries = fs::read_dir(path)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let mut traversal_res = self.get_csv_files(path.to_str().unwrap())?;
                csv_file_paths.append(&mut traversal_res);
            }
            let extension = path.extension().unwrap_or_default();
            if extension.to_ascii_lowercase() == "csv" {
                csv_file_paths.push(path);
            }
        }
        Ok(csv_file_paths)
    }
}

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Insert(InsertConfig),
    Reorder(ReorderConfig),
}

#[derive(Args, Debug, Clone)]
struct InsertConfig {
    #[arg(long)]
    path: String,
    #[arg(long)]
    column: String,
    #[arg(long)]
    default_value: String,
    #[arg(long)]
    order: i32,
}

#[derive(Args, Debug, Clone)]
struct ReorderConfig {
    #[arg(long)]
    path: String,
    #[arg(long)]
    column: String,
    #[arg(long)]
    order: i32,
}

fn main() {
    let cli = Cli::parse();
    run(cli).unwrap_or_else(|_| println!("{}", "Migration failed".red()));
    println!("{}", "Migration done".green());
}

fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    match cli.command {
        Commands::Insert(insert_config) => InsertMigration::new(insert_config).run().unwrap(),
        Commands::Reorder(reorder_config) => ReorderMigration::new(reorder_config).run().unwrap(),
    };

    Ok(())
}

#[derive(Clone)]
struct InsertMigration {
    config: InsertConfig,
}
impl Migration for InsertMigration {
    type ConfigType = InsertConfig;

    fn new(config: Self::ConfigType) -> Self {
        Self { config }
    }

    fn run(&self) -> Result<(), Box<dyn Error>> {
        let InsertConfig {
            path,
            column,
            default_value,
            order,
        } = &self.config;
        println!(
            "Inserting {} with default value {} as #{} in path {}",
            &column.blue(),
            &default_value.blue(),
            &order.to_string().blue(),
            &path.blue()
        );
        let files = self.get_csv_files(&path)?;
        for file in files {
            println!("Migrating {:?}", &file);
            self.insert_column(&file, &column, &default_value, *order)?;
        }

        Ok(())
    }
}

impl InsertMigration {
    fn insert_column(
        &self,
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
}

struct ReorderMigration {
    config: ReorderConfig,
}
impl Migration for ReorderMigration {
    type ConfigType = ReorderConfig;

    fn new(config: Self::ConfigType) -> Self {
        Self { config }
    }

    fn run(&self) -> Result<(), Box<dyn Error>> {
        let ReorderConfig {
            path,
            column,
            order,
        } = &self.config;
        println!(
            "Reordering {} to #{} path {}",
            &column.blue(),
            &order.to_string().blue(),
            &path.blue()
        );

        let files = self.get_csv_files(&path)?;
        for file in files {
            println!("Migrating {:?}", &file);
            self.shift_column(&file, &column, *order)?;
        }
        Ok(())
    }
}

impl ReorderMigration {
    fn shift_column(
        &self,
        path: &PathBuf,
        column: &String,
        order: i32,
    ) -> Result<(), Box<dyn Error>> {
        let mut content = String::new();
        File::open(path)?.read_to_string(&mut content)?;
        let mut reader = csv::Reader::from_reader(content.as_bytes());
        let mut writer = csv::Writer::from_path(path)?;

        // headers
        let original_headers = reader.headers()?.clone();
        let mut new_headers = StringRecord::new();
        let target_header_index = original_headers
            .iter()
            .position(|h| h == column)
            .expect("Column not found");
        if target_header_index as i32 == order - 1 {
            println!(
                "{}",
                format!("Column {} already on #{}", column, order).yellow()
            );
            writer.write_record(&original_headers.clone())?;
            for r in reader.records() {
                writer.write_record(&r.unwrap())?;
            }
            return Ok(());
        }

        let target_header = original_headers.get(target_header_index).unwrap();
        let mut headers_vec: Vec<&str> = original_headers.iter().collect();
        headers_vec.remove(target_header_index);
        let headers: StringRecord = headers_vec.into();
        for (i, header) in headers.iter().enumerate() {
            if i as i32 == order - 1 {
                new_headers.push_field(target_header);
            }
            new_headers.push_field(header);
        }
        writer.write_record(&new_headers)?;

        // values
        for original_record in reader.records() {
            let original_record = original_record?;
            let target_value = original_record
                .get(target_header_index)
                .expect("Value to migrate not found");
            let mut record = original_record.iter().collect::<Vec<&str>>();
            record.remove(target_header_index);
            let mut new_record = StringRecord::new();
            let record_iter = record.iter().enumerate();
            for (j, value) in record_iter {
                if j as i32 == (order - 1) {
                    new_record.push_field(target_value);
                    new_record.push_field(value);
                } else {
                    new_record.push_field(value);
                }
            }
            writer.write_record(&new_record)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_insert_column() {
        let test_dir = "test_files/insert";
        fs::remove_dir_all(test_dir).unwrap();
        fs::create_dir_all(test_dir).unwrap();
        let mut path = PathBuf::new();
        path.push(format!("{}/test.csv", test_dir));
        let mut file = File::create(path.clone()).unwrap();
        file.write_all(
            b"H1,H2,H3,H4,H5,H6,H7,H8,H9\nA1,A2,A3,A4,A5,A6,A7,A8,A9\nB1,B2,B3,B4,B5,B6,B7,B8,B9",
        )
        .unwrap();

        let cli = Cli {
            command: Commands::Insert(InsertConfig {
                path: test_dir.to_string(),
                column: "H_new".to_string(),
                default_value: "V_new".to_string(),
                order: 3,
            }),
        };
        run(cli).unwrap();
        let mut modified_file = File::open(path.clone()).unwrap();
        let mut modified_content = String::new();
        modified_file.read_to_string(&mut modified_content).unwrap();
        assert_eq!(
            modified_content,
            String::from(
                "H1,H2,H_new,H3,H4,H5,H6,H7,H8,H9\nA1,A2,V_new,A3,A4,A5,A6,A7,A8,A9\nB1,B2,V_new,B3,B4,B5,B6,B7,B8,B9\n"
            )
        )
    }

    #[test]
    fn test_reorder_column() {
        let reorder_test_cases = vec![
            ("H1,H2,H3,H4,H5,H6,H7,H8,H9\nA1,A2,A3,A4,A5,A6,A7,A8,A9\nB1,B2,B3,B4,B5,B6,B7,B8,B9".to_string(), "H3,H1,H2,H4,H5,H6,H7,H8,H9\nA3,A1,A2,A4,A5,A6,A7,A8,A9\nB3,B1,B2,B4,B5,B6,B7,B8,B9\n".to_string(), "H3", 1),
            ("H1,H2,H3,H4,H5,H6,H7,H8,H9\nA1,A2,A3,A4,A5,A6,A7,A8,A9\nB,B,B,B4,B5,B6,B7,B8,B9".to_string(), "H3,H1,H2,H4,H5,H6,H7,H8,H9\nA3,A1,A2,A4,A5,A6,A7,A8,A9\nB,B,B,B4,B5,B6,B7,B8,B9\n".to_string(), "H3", 1),
            ("H1,H2,H3,H4,H5,H6,H7,H8,H9\nA1,A2,A3,A4,A5,A6,A7,A8,A9\nB1,B2,B3,B4,B5,B6,B7,B8,B9".to_string(), "H1,H2,H3,H4,H5,H6,H7,H8,H9\nA1,A2,A3,A4,A5,A6,A7,A8,A9\nB1,B2,B3,B4,B5,B6,B7,B8,B9\n".to_string(), "H1", 1),
            ("H1,H2,H3,H4,H5,H6,H7,H8,H9\nA1,A2,A3,A4,A5,A6,A7,A8,A9\nB1,B2,B3,B4,B5,B6,B7,B8,B9".to_string(), "H2,H1,H3,H4,H5,H6,H7,H8,H9\nA2,A1,A3,A4,A5,A6,A7,A8,A9\nB2,B1,B3,B4,B5,B6,B7,B8,B9\n".to_string(), "H1", 2),

        ];

        let test_dir = "test_files/reorder";
        fs::remove_dir_all(test_dir).unwrap();
        fs::create_dir_all(test_dir).unwrap();
        for (i, tc) in reorder_test_cases.iter().enumerate() {
            let (init, expected, column, order) = tc;
            let mut path = PathBuf::new();
            path.push(format!("{}/test_{}.csv", test_dir, i));
            let mut file = File::create(path.clone()).unwrap();
            let buff = init.clone().into_bytes();
            file.write_all(&buff).unwrap();

            let cli = Cli {
                command: Commands::Reorder(ReorderConfig {
                    path: test_dir.to_string(),
                    column: column.to_string(),
                    order: *order,
                }),
            };
            run(cli).unwrap();
            let mut modified_file = File::open(path.clone()).unwrap();
            let mut modified_content = String::new();
            modified_file.read_to_string(&mut modified_content).unwrap();
            assert_eq!(modified_content, *expected)
        }
    }
}
