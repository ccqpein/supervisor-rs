use chrono::prelude::*;

pub fn timelog(s: &str) -> String {
    let mut dt = Local::now().format("[%Y-%m-%d %H:%M:%S] ").to_string();
    dt.push_str(s);
    dt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_print() {
        println!("{}", timelog("aaaa"));
    }
}
