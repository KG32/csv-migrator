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
        self.insert(self.config.clone())?;
        Ok(())
    }
}

impl InsertMigration {
    fn insert(&self, config: InsertConfig) -> Result<(), Box<dyn Error>> {
        let InsertConfig {
            path,
            column,
            default_value,
            order,
        } = config;
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
            self.insert_column(&file, &column, &default_value, order)?;
        }

        Ok(())
    }

    fn insert_column(
        &self,
        path: &PathBuf,
        column: &String,
        default_value: &String,
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
        todo!();
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
        fs::create_dir_all(test_dir).unwrap();
        let mut path = PathBuf::new();
        path.push(format!("{}/test.csv", test_dir));
        let mut file = File::create(path.clone()).unwrap();
        file.write_all(
            b"H1,H2,H3,H4,H5,H6,H7,H8,H9\nA1,A2,A3,A4,A5,A6,A7,A8,A9\nB11,B22,B33,B44,B55,B66,B77,B88,B99",
        ).unwrap();

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
                "H1,H2,H_new,H3,H4,H5,H6,H7,H8,H9\nA1,A2,V_new,A3,A4,A5,A6,A7,A8,A9\nB11,B22,V_new,B33,B44,B55,B66,B77,B88,B99\n"
            )
        )
    }

    #[test]
    fn test_reorder_column() {
        let test_dir = "test_files/reorder";
        fs::create_dir_all(test_dir).unwrap();
        let mut path = PathBuf::new();
        path.push(format!("{}/test.csv", test_dir));
        let mut file = File::create(path.clone()).unwrap();
        file.write_all(
            b"H1,H2,H3,H4,H5,H6,H7,H8,H9\nA1,A2,A3,A4,A5,A6,A7,A8,A9\nB11,B22,B33,B44,B55,B66,B77,B88,B99",
        ).unwrap();

        let cli = Cli {
            command: Commands::Reorder(ReorderConfig {
                path: test_dir.to_string(),
                column: "H3".to_string(),
                order: 1,
            }),
        };
        run(cli).unwrap();
        let mut modified_file = File::open(path.clone()).unwrap();
        let mut modified_content = String::new();
        modified_file.read_to_string(&mut modified_content).unwrap();
        assert_eq!(
            modified_content,
            String::from(
                "H3,H1,H2,H4,H5,H6,H7,H8,H9\nA3,A1,A2,A4,A5,A6,A7,A8,A9\nB33,B11,B44,B55,B66,B77,B88,B99\n"
            )
        )
    }
}
