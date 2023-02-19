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

    if argv.len() < 5 {
        eprintln!("Not enough arguments for alias.");
        return;
    }
    
    let key = &argv[2];

    if argv[3].as_str() != "=" {
        eprintln!("Equal sign (=) needed for alias.");
        return;
    }
    let mut args: Vec<String> = Vec::new();
    let cmd;
    if argv[4].contains("'") {
        let string = argv[4].clone().drain(1..argv[4].len()-1).collect::<String>();
        let temp_args:Vec<&str> = string.split(" ").collect();

        for arg in temp_args.iter() {
            args.push(arg.to_string());
        }
        cmd = args.drain(..1).collect::<String>().clone();


    }
    else {
        cmd = argv[4].clone();
    }

    aliases.insert(key.to_string(), (cmd,args));
}

pub fn export(argv: &Vec<String>) {
    if argv.len() < 5 {
        eprintln!("Not enough argument for exporting.");
        return;
    }

    let key = &argv[2];
    if argv[3].as_str() != "=" {
        eprintln!("Equal sign (=) needed for exporting")
    }
    
    let args; //argv.clone().drain(3..).collect::<Vec<String>>().join(" ");
    if argv[4].contains("'") {
       args = argv[4].clone().drain(1..argv[4].len()-1).collect(); 
    }
    else {
        args = argv[4].clone();
    }

    env::set_var(key,args);    
}

pub fn variable(argv: &Vec<String>, variables: &mut BTreeMap<String,String>) {
    let key = &argv[0];
    if argv[1].as_str() != "=" {
        eprintln!("Equal sign (=) needed for assigning variable.")
    }
    
    let args;
    if argv[2].contains("'") {
        args = argv[2].clone().drain(1..argv[2].len()-1).collect();
    }
    else {
        args = argv[2].as_str().to_string();
    }
    
    variables.insert(key.to_string(),args);
}


pub fn print_vars(variables: &mut BTreeMap<String,String>) {
    
        for (key, value) in variables.iter() {
           print!("{} = {} ",key,value); 
           println!("");
        }
        return;

}
