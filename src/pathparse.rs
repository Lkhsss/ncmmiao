use std::path::Path;
use walkdir::WalkDir;

pub fn pathparse(input: Vec<String>) -> Vec<String> {
    let mut undumpfile = Vec::new(); // 该列表将存入文件的路径
                                     // 遍历输入的每一个路径参数
    for arg in input {
        //解析传入的每一个路径：文件or文件夹
        let path = Path::new(&arg);

        if path.is_file() {
            // 当后缀符合为ncm时才加入列表
            match path.extension() {
                Some(extension) => {
                    if extension == "ncm" {
                        let _ = &mut undumpfile.push(arg.to_owned());
                    }
                }
                None => {}
            }
        } else if path.is_dir() {
            for entry in WalkDir::new(path) {
                let new_entry = entry.unwrap().clone();
                let filepath = new_entry.into_path();
                // 当后缀符合为ncm时才加入列表
                match filepath.extension() {
                    Some(extension) => {
                        if extension == "ncm" {
                            let _ = &mut undumpfile.push(String::from(filepath.to_str().unwrap()));
                        }
                    }
                    None => {
                        continue;
                    }
                }
            }
        }
    }
    undumpfile
}
