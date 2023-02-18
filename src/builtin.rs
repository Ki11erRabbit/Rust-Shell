use std::env;
use std::path::Path;
use std::collections::BTreeMap;

pub fn change_dir(argv: &Vec<String>) {
    let path;
    if argv.len() == 1 {
        let key = "HOME";
        match env::var(key) {
            Err(_) => {
                eprintln!("User's home not set!");
                return;
            }
            Ok(val) => {
                path = Path::new(&val);

                match env::set_current_dir(path) {
                    Ok(_) => (),
                    Err(e) => eprintln!("{}",e),
                }

                return;

            }
        }
    }
    else {
        path = Path::new(&argv[1]);
    }

    match env::set_current_dir(path) {
        Ok(_) => (),
        Err(_) => eprintln!("cd: no such file or directory: {}",argv[1]),
    }
}

pub fn alias(argv: &Vec<String>, aliases: &mut BTreeMap<String,(String,Vec<String>)>) {

    if argv.len() == 1 {
        for (key, value) in aliases.iter() {
           print!("{} = {} ",key,value.0); 
           for arg in value.1.iter() {
            print!("{} ",arg);
           }
           println!("");
        }
        return;
    }

    if argv.len() < 4 {
        eprintln!("Not enough arguments for alias.");
        return;
    }
    
    let key = &argv[1];

    if argv[2].as_str() != "=" {
        eprintln!("Equal sign (=) needed for alias.");
        return;
    }
    let mut args: Vec<String> = argv.clone().drain(3..).collect();
    let cmd = args[0].clone();
    if args.len() > 1 {
        args = args.drain(1..).collect();
    }
    else {
        args = Vec::new();
    }

    aliases.insert(key.to_string(), (cmd.to_string(),args));
    

}
