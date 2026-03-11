use std::fs;

fn main() {
    let dir = "src/bin";
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() && path.extension().unwrap() == "rs" {
            let content = fs::read_to_string(&path).unwrap();
            if !content.starts_with("#![allow(warnings)]") {
                let new_content = format!("#![allow(warnings)]\n{}", content);
                fs::write(&path, new_content).unwrap();
            }
        }
    }
}
