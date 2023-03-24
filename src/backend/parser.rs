use super::csv_handler::DataEntry;
use chrono::prelude::{NaiveDate, NaiveDateTime};
use std::{error::Error, num::ParseFloatError, num::ParseIntError, sync::Arc};

pub fn parse(cell: &mut DataEntry) {
    let sql_type: String;
    let type_args: Option<String>;
    /* extract sql type and args */
    if let Some(arg_index) = cell
        .curr_field_description
        .as_ref()
        .unwrap()
        .field_type
        .find("(")
    {
        sql_type = cell
            .curr_field_description
            .as_ref()
            .unwrap()
            .field_type
            .get(0..arg_index)
            .unwrap()
            .to_owned();

        if let Some(args_res) = cell
            .curr_field_description
            .as_ref()
            .unwrap()
            .field_type
            .get(
                arg_index
                    ..cell
                        .curr_field_description
                        .as_ref()
                        .unwrap()
                        .field_type
                        .len(),
            )
        {
            type_args = Some(args_res.to_owned());
        } else {
            type_args = None;
        }
    } else {
        sql_type = cell
            .curr_field_description
            .as_ref()
            .unwrap()
            .field_type
            .to_owned();
        type_args = None;
    }

    match sql_type.as_str() {
        "char" => parse_char(cell, type_args),
        "varchar" => parse_varchar(cell, type_args),
        "text" => parse_text(cell, type_args),
        "double" => parse_double(cell, type_args),
        "decimal" => parse_decimal(cell, type_args),
        "int" => parse_int(cell, type_args),
        "tinyint" => parse_tinyint(cell, type_args),
        "date" => parse_date(cell),
        "datetime" => parse_datetime(cell),
        "enum" => parse_enum(cell, type_args),
        _ => {
            let arc: Arc<dyn Error + Send + Sync> = Arc::from(<String as Into<
                Box<dyn Error + Send + Sync>,
            >>::into(sql_type));
            cell.is_parsed = Some(Err(arc))
        }
    }
}

fn process_args(args: Option<String>) -> Option<Vec<u32>> {
    if let Some(args) = args {
        //remove the "()" at start and end
        let mut chargs = args.trim().chars();
        chargs.next();
        chargs.next_back();

        //separate by ,
        let args: Vec<&str> = chargs.as_str().split(",").collect();
        let mut num_args: Vec<u32> = Vec::new();

        //if first char is `"`, it's an enum so handle differently
        if args[0].chars().next().unwrap() == '"' {
            todo!()
        } else {
            //convert to number arguments,
            for num_str in args.iter() {
                let num_char: Result<u32, ParseIntError> = num_str.parse();
                if let Ok(num_char) = num_char {
                    num_args.push(num_char);
                }
            }
        }
        return Some(num_args);
    } else {
        return None;
    }
}
fn parse_char(cell: &mut DataEntry, args: Option<String>) {
    //CHAR(5) = 'chart ' - CHAR adds spaces to values on the right to the specified length
    if let Some(num_args) = process_args(args) {
        let count = cell.data.chars().count() as u32;
        if count > num_args[0] {
            cell.is_parsed = Some(Ok(()));
        } else {
            cell.is_parsed = Some(Err(Arc::from(<&str as Into<
                Box<dyn Error + Send + Sync>,
            >>::into("Too long"))))
        }
    }
}

fn parse_varchar(cell: &mut DataEntry, args: Option<String>) {
    //VARCHAR(5) = 'chart'
    if let Some(num_args) = process_args(args) {
        let count = cell.data.chars().count() as u32;
        if count <= num_args[0] {
            cell.is_parsed = Some(Ok(()))
        } else {
            cell.is_parsed = Some(Err(Arc::from(<&str as Into<
                Box<dyn Error + Send + Sync>,
            >>::into("Too long"))))
        }
    }
}
fn parse_text(cell: &mut DataEntry, args: Option<String>) {
    // max chars:  65535
    cell.is_parsed = Some(Ok(()));
}
fn parse_double(cell: &mut DataEntry, args: Option<String>) {
    //Double(6,2) = 9999.99
    if let Some(num_args) = process_args(args) {
        let num_parse_res: Result<f32, ParseFloatError> = cell.data.parse();
        if let Ok(_) = num_parse_res {
            let dot_i = cell.data.find(".").unwrap();
            // -1 cuz the dot is there
            let count_to_dot = cell.data.chars().count() as u32;
            let count_from_dot = cell.data.chars().skip(dot_i + 1).count() as u32;
            if count_to_dot - 1 <= num_args[0] && count_from_dot > num_args[1] {
                cell.is_parsed = Some(Ok(()));
            } else {
                cell.is_parsed = Some(Err(Arc::from(<&str as Into<
                    Box<dyn Error + Send + Sync>,
                >>::into("too many numbers"))))
            }
        }
    } else {
        cell.is_parsed = Some(Err(Arc::from(<&str as Into<
            Box<dyn Error + Send + Sync>,
        >>::into("Not a number"))))
    }
}
fn parse_decimal(cell: &mut DataEntry, args: Option<String>) {
    //DECIMAL(6,2) = 9999.99
    if let Some(num_args) = process_args(args) {
        let num_parse_res: Result<f32, ParseFloatError> = cell.data.parse();
        if let Ok(_) = num_parse_res {
            let dot_i = cell.data.find(".").unwrap();
            // -1 cuz the dot is there
            let count_to_dot = cell.data.chars().count() as u32;
            let count_from_dot = cell.data.chars().skip(dot_i + 1).count() as u32;
            if count_to_dot - 1 <= num_args[0] && count_from_dot >= num_args[1] {
                cell.is_parsed = Some(Ok(()));
            } else {
                cell.is_parsed = Some(Err(Arc::from(<&str as Into<
                    Box<dyn Error + Send + Sync>,
                >>::into("too many numbers"))))
            }
        } else {
            cell.is_parsed = Some(Err(Arc::from(<&str as Into<
                Box<dyn Error + Send + Sync>,
            >>::into("Not a number"))))
        }
    }
}
fn parse_int(cell: &mut DataEntry, args: Option<String>) {
    let num_parse_res: Result<i32, ParseIntError> = cell.data.parse();
    if let Ok(_) = num_parse_res {
        cell.is_parsed = Some(Ok(()));
    } else {
        cell.is_parsed = Some(Err(Arc::from(<&str as Into<
            Box<dyn Error + Send + Sync>,
        >>::into(
            "Number too big or not a number."
        ))))
    }
}
fn parse_tinyint(cell: &mut DataEntry, args: Option<String>) {
    //max val 255 or -128 to 127 signed
    let num_parse_res: Result<i8, ParseIntError> = cell.data.parse();
    if let Ok(_) = num_parse_res {
        cell.is_parsed = Some(Ok(()))
    } else {
        cell.is_parsed = Some(Err(Arc::from(<&str as Into<
            Box<dyn Error + Send + Sync>,
        >>::into(
            "Number too big or not a number."
        ))))
    }
}
fn parse_date(cell: &mut DataEntry) {
    //YYYY-MM-DD from ‘1000-01-01’ to ‘9999-12-31’,also ’0000-00-00’
    if cell.data == "0000-00-00" {
        cell.is_parsed = Some(Ok(()))
    } else {
        match NaiveDate::parse_from_str(&cell.data, "%Y-%m-%d") {
            Ok(_) => cell.is_parsed = Some(Ok(())),
            Err(e) => {
                cell.is_parsed = Some(Err(Arc::from(<String as Into<
                    Box<dyn Error + Send + Sync>,
                >>::into(format!(
                    "Invalid date: {}",
                    e
                )))))
            }
        }
    }
}
fn parse_datetime(cell: &mut DataEntry) {
    //YYYY-MM-DD HH:MM:SS, also ’0000-00-00’
    if cell.data == "0000-00-00 00:00:00" {
        cell.is_parsed = Some(Ok(()))
    } else {
        match NaiveDateTime::parse_from_str(&cell.data, "%Y-%m-%d %H:%M:%S") {
            Ok(_) => cell.is_parsed = Some(Ok(())),
            Err(e) => {
                cell.is_parsed = Some(Err(Arc::from(<String as Into<
                    Box<dyn Error + Send + Sync>,
                >>::into(format!(
                    "Invalid date: {}",
                    e
                )))))
            }
        }
    }
}
fn parse_enum(cell: &mut DataEntry, args: Option<String>) {
    todo!()
}
