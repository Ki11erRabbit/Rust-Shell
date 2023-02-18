use std::process::{self,Command, Stdio, Child};
use std::env;
use std::io::{self,Write};
use std::fs::File;
use std::os::unix::process::CommandExt;
use std::sync::Mutex;
use std::fmt;
use signal_hook::{consts::*, iterator::Signals};
use std::thread;
use nix::unistd::Pid;
use nix::unistd::pause;
use nix::sys::signal::{self, Signal};
use nix::sys::wait;

pub enum ProccessState {
    UNDEF,
    FG,
    BG,
    ST
}

impl fmt::Display for ProccessState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProccessState::UNDEF => write!(f,"UNDEF"),
            ProccessState::FG => write!(f,"FG"),
            ProccessState::BG => write!(f,"BG"),
            ProccessState::ST => write!(f,"ST"),
        }
    }
}

pub struct Job {
    pid: i32,
    pgid: i32,
    jid: u32,
    state: ProccessState,
    cmdline: String,
    pipeline: Mutex<Vec<Child>>

}

impl Job {
    pub fn new(pid: i32, pgid: i32, jid: u32, state: ProccessState, cmdline: String, pipeline: Mutex<Vec<Child>>) -> Self {
        Self {pid: pid, pgid: pgid, jid: jid, state: state, cmdline: cmdline, pipeline: pipeline}
    }
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result = write!(f,"[{}] ({}) ",self.jid,self.pid);
        if result == Err(std::fmt::Error) {
            return result;
        }
        let result = match self.state {
            ProccessState::FG => write!(f,"Foreground "),
            ProccessState::BG => write!(f,"Running "),
            ProccessState::ST => write!(f,"Stopped "),
            ProccessState::UNDEF =>  write!(f,"listjobs: Internal error: job[{}].state={} ",self.jid,self.state),
        };
        if result == Err(std::fmt::Error) {
            return result;
        }
        write!(f,"{}",self.cmdline)
    }
}

pub struct Jobs {
    jobs: Vec<Job>,
    next_jid: u32,
}

impl Jobs {
    pub const fn new() -> Self {
        Self {jobs: Vec::new(),next_jid: 1}
    }

    pub fn addjob(&mut self, pid: i32, pgid: i32, state: ProccessState, cmdline: String, pipeline: Mutex<Vec<Child>>) {
       self.jobs.push(Job::new(pid,pgid,self.next_jid,state,cmdline,pipeline)); 
       self.next_jid += 1;
    }

    pub fn delete_job(&mut self,pid: i32) -> Result<&str,&str> {
        if pid < 1 {
            return Err("Invalid PID");
        }

        for i in 0..self.jobs.len() {
            if self.jobs[i].pid == pid {
                self.jobs.remove(i);
                self.set_next_jid();
                return Ok("Successfully removed job");
            }
        }
        return Err("Invalid PID");
    }

    fn set_next_jid(&mut self) {
        let mut max = 0;
        for job in self.jobs.iter() {
           if job.jid > max {
                max = job.jid;
           } 
        }
        self.next_jid = max + 1;
    }

    pub fn get_job_pid(&mut self, pid: i32) -> Option<&mut Job> {
        for job in self.jobs.iter_mut() {
            if job.pid == pid {

                return Some(job);
            }
        
        }
        return None;
    }

    pub fn get_job_jid(&mut self, jid: u32) -> Option<&mut Job> {
        if jid > self.next_jid || jid <= 0 {
            return None;
        } 
        
        return Some(&mut self.jobs[jid as usize - 1]);
    }
    

    pub fn iter(&self) -> std::slice::Iter<Job> {
        self.jobs.iter()
    }
    pub fn iter_mut(&mut self) -> std::slice::IterMut<Job> {
        self.jobs.iter_mut()
    }
}

impl fmt::Display for Jobs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for job in self.jobs.iter() {
            
            let result = write!(f,"{}",job);

            if result == Err(std::fmt::Error) {
                return result;
            }
        }
        Ok(())
    }
}


const PROMPT: &str = "tsh> ";
static mut VERBOSE:i32 = 0;
static mut JOBS: Jobs = Jobs::new();




fn main() {
    let args: Vec<String> = env::args().collect();
    let mut emit_prompt = 1;
   
    if args.len() > 1 {

        if args[1].contains("-") {
            let mut bad_input = true;
            if args[1].contains("h") {
                usage();
            }
            if args[1].contains("v") {
                unsafe { VERBOSE = 1 };
                bad_input = false;
            }
            if args[1].contains("p") {
                emit_prompt = 0;
                bad_input = false;
            }
            if bad_input {
                usage();
            }
        }
    }    

   let signals = Signals::new(&[SIGINT,SIGCHLD,SIGTSTP]);
    thread::spawn(move || {
        for sig in &mut signals.unwrap(){

            if sig == SIGINT {
                if unsafe {VERBOSE} == 1 {
                    println!("sigint_handler");
                }
                
                for job in unsafe {JOBS.iter_mut()} {
                    
                    match job.state {
                        ProccessState::FG => {
                                
                                signal::kill(Pid::from_raw(-job.pid),Signal::SIGINT).unwrap();
                                unsafe {
                                    match JOBS.delete_job(job.pid) {
                                        Err(e) => eprintln!("{}",e),
                                        Ok(_) => (),
                                    }
                                }

                        }
                        _ => {
                                continue;
                        }
                    }

                }

            }
            else if sig == SIGCHLD {
                if unsafe {VERBOSE} == 1 {
                    println!("sigchild_handler");
                }
                let flags: wait::WaitPidFlag = wait::WaitPidFlag::WNOHANG | wait::WaitPidFlag::WUNTRACED;

                loop {
                    match wait::waitpid(Pid::from_raw(-1), Some(flags)) {
                        Err(_) => break,
                        Ok(x) => {
                            match x {
                                wait::WaitStatus::StillAlive => break,
                                wait::WaitStatus::Exited(pid,_status) => {
                                    unsafe { 
                                        match JOBS.delete_job(pid.as_raw()) {
                                            Ok(_) => (),
                                            Err(_) => (),
                                        } 
                                    };

                                }
                                wait::WaitStatus::Signaled(pid, signal, _core_dump) => {
                                    let job;
                                    unsafe { 
                                        match JOBS.get_job_pid(pid.as_raw()) {
                                            Some(x) => {
                                                job = x;
                                                println!("Job [{}] ({}) terminated by signal {}",job.jid,pid,signal);
                                            },
                                            None => (),
                                        } 
                                    };

                                    unsafe { 
                                        match JOBS.delete_job(pid.as_raw()) {
                                            Ok(_) => (),
                                            Err(_) => (),
                                        } 
                                    };
                                },
                                wait::WaitStatus::Stopped(pid,signal) => {
                                    let mut job;
                                    unsafe { job = JOBS.get_job_pid(pid.as_raw()).unwrap(); };
                                    
                                    job.state = ProccessState::ST;
                                     
                                    
                                    println!("Job [{}] ({}) stopped by signal {}",job.jid,pid,signal);
                                }
                                _ => (),
                            }
                        }

                    }
                }
                        
                    
                


            }
            else if sig == SIGTSTP {

                if unsafe {VERBOSE} == 1 {
                    println!("sigtstp_handler");
                }
                
                for job in unsafe {JOBS.iter_mut()} {
                    
                    match job.state {
                        ProccessState::FG => {

                                signal::kill(Pid::from_raw(-job.pid),Signal::SIGTSTP).unwrap();        
                        }
                        _ => {
                                continue;
                        }
                    }

                }

            }
        }
    });


    loop {
        let mut buffer = String::new();
        if emit_prompt == 1 {
            print!("{}",PROMPT);
            io::stdout().flush().unwrap();
        }
        io::stdin().read_line(&mut buffer)
            .expect("Failed to read line");

        eval(buffer);
    }
}


fn eval(cmdline: String) {
    if unsafe { VERBOSE == 1 } {
        println!("Eval");
    }
    let argv: Vec<String>;
    let bg: i32;
    let pair = parseline(&cmdline);
    bg = pair.0;
    argv = pair.1;
    
    if unsafe { VERBOSE == 1 } {
        println!("{:?}",argv);
    }

    if builtin_cmd(&argv) == 1 {
        return;
    }

    let set = parseargs(&argv);

    let cmds = set.0;
    let args = set.1;
    let stdin_redir = set.2;
    let stdout_redir = set.3;

    if unsafe { VERBOSE == 1 } {
        println!("{:?}",cmds);
        println!("{:?}",args);
        println!("{:?}",stdin_redir);
        println!("{:?}",stdout_redir);
        
        println!("\npid = {}", process::id());
    }
    
    let mut processes: Vec<Child> = Vec::new(); 
    let mut group_id = 0;
    for i in 0..cmds.len() {
        let mut command: &mut Command = &mut Command::new(cmds[i].as_str());
        command = command.process_group(group_id);
        //println!("{}",cmds[i].len());
        command = command.args(args[i].as_slice());
        if stdout_redir[i] == usize::MAX {
            command = command.stdout(Stdio::piped());
        }
        else if stdout_redir[i] != usize::MAX && stdout_redir[i] != usize::MAX -1{
            let file = File::create(argv[stdout_redir[i]].as_str()).expect("Bad file path");
            command = command.stdout(file);
        }
        if stdin_redir[i] == usize::MAX {
            command = command.stdin(processes[i-1].stdout.take().unwrap());
        }
        else if stdin_redir[i] != usize::MAX && stdin_redir[i] != usize::MAX -1{
            let file = File::open(argv[stdin_redir[i]].as_str()).expect("Bad file path");
            command = command.stdin(file);
        } 
        

        match command.spawn() {
            Ok(x) => processes.push(x),
            Err(y) => eprintln!("tsh: command not found: {} {}", cmds[i],y)
        }
        if unsafe { VERBOSE == 1 } {
            
            println!("pid child = {}", processes[i].id());
        }
        

        if i == 0 {
            group_id = processes[i].id().try_into().unwrap();
        }
    }
    
    let pid = group_id.try_into().unwrap();
    if bg == 0 {
        if unsafe { VERBOSE == 1 } {
            println!("spawning in forground");
        }
        unsafe {
            JOBS.addjob(pid, pid, ProccessState::FG, cmdline, Mutex::new(processes));
        } 
        waitfg(pid);
    }
    else if bg == 1 {
        if unsafe { VERBOSE == 1 } {
            println!("spawning in background");
        }

        unsafe {
            JOBS.addjob(pid, pid, ProccessState::BG, cmdline, Mutex::new(processes));
        } 

    }


/*
    for i in 0..processes.len() {
        match processes[i].wait() {
            Ok(_status) => (),
            Err(error) => eprintln!("{}", error)
        }
    }*/

}

fn parseline(cmdline: &String) -> (i32,Vec<String>) {
    if unsafe { VERBOSE == 1 } {
        println!("Parseline");
    }
    let mut argv: Vec<String> = Vec::new();
    let bg: i32;
    let mut array = cmdline.clone(); 
    array.pop();
    array.push(' ');

    if cmdline.rfind("&") != None {
        bg = 1;
    }
    else {
        bg = 0;
    }

    while array.len() != 0 {
        //println!("{}",array);
        //println!("array len: {}", array.len());
        match array.get(..1).unwrap() {
            "'" => {
                        let mut temp: String = array.drain(..1).collect();
                        //println!("{}!", array);
                        let temp2: String = array.drain(..array.find('\'').unwrap()+1).collect();
                        temp += &temp2;

                        argv.push(temp);
                   },
            " " => {
                        //println!("Space");
                        array.drain(..1);
                   },
            "|" => {
                        //println!("Space");
                        argv.push(array.drain(..1).collect());
                   },
            "<" => {
                        //println!("Space");
                        argv.push(array.drain(..1).collect());
                   },
            ">" => {
                        //println!("Space");
                        argv.push(array.drain(..1).collect());
                   },
            "&" => {
                        //println!("Space");
                        array.drain(..1);
                   },
            _ => {
                        //println!("Default");
                        argv.push(array.drain(0..array.find(|c: char| c == '>' || c == '|' || c == '<' || c == ' ').unwrap()).collect());

                 } 
        }

        //println!("{:?}",argv);

    }


    return (bg,argv);
}

fn parseargs(argv: &Vec<String>) -> (Vec<String>,Vec<Vec<String>>,Vec<usize>,Vec<usize>) {
    if unsafe { VERBOSE == 1 } {
        println!("parseargs");
    }
    let mut cmds: Vec<String> = Vec::new();
    let mut args: Vec<Vec<String>> = Vec::new();
    let mut stdin_redir: Vec<usize> = Vec::new();
    let mut stdout_redir: Vec<usize> = Vec::new();

    let mut curr_cmd = 0;
    cmds.push("".to_string());
    args.push(Vec::new());
    stdin_redir.push(usize::MAX -1);
    stdout_redir.push(usize::MAX -1);

    let mut skip = false;
    for i in 0..argv.len() {
        match argv[i].as_str() {
            "|" => {
                    stdout_redir[curr_cmd] = usize::MAX;
                    stdin_redir.push(usize::MAX);
                    stdout_redir.push(usize::MAX-1);
                    cmds.push("".to_string());
                    args.push(Vec::new());
                    curr_cmd += 1;
                },
            "<" => {
                    //stdin_redir[curr_cmd] = args[curr_cmd].len();
                    stdin_redir[curr_cmd] = i + 1;
                    skip = true;
                },
            ">" => {
                    //stdout_redir[curr_cmd] = args[curr_cmd].len();
                    stdout_redir[curr_cmd] = i + 1;
                    skip = true;
                },
            _ => {
                    if skip {
                        skip = false;
                        continue;
                    }
                    if cmds[curr_cmd].as_str() == "" {
                        cmds[curr_cmd] = argv[i].as_str().to_string();
                    }
                    else {
                        args[curr_cmd].push(argv[i].as_str().to_string());
                    }
                }
        } 

    }
    


    return (cmds,args,stdin_redir,stdout_redir);
}

fn do_bgfg(argv: &Vec<String>) {
    if argv.len() == 1 {
        println!("{} command requires PID or %jobid argument",argv[0]);
        return;
    }

    if argv[0].as_str() == "fg" {
        let result;
        let num;
        let mut jid = false;
        if argv[1].find("%") != None {
            jid = true;
            result = argv[1].clone().drain(1..).collect::<String>().parse::<u32>();
        }
        else {
            result = argv[1].clone().drain(..).collect::<String>().parse::<u32>();
        }

        match result {
            Ok(val) => num = val,
            Err(_) => {
                eprintln!("{}: argument must be a PID or %jobid",argv[0]);
                return;
            },
        }
        
        let job: Option<&mut Job>;
        if jid {
            job = unsafe { JOBS.get_job_jid(num) };
        }
        else {
            job = unsafe { JOBS.get_job_pid(num as i32) };
        }

        match job {
            Some(job) => {
                job.state = ProccessState::FG;
                signal::kill(Pid::from_raw(-job.pgid),Signal::SIGCONT).unwrap();

                waitfg(job.pgid);
            },
            None => {
                if jid {
                    eprintln!("%{}: No such job",num);
                }
                else {
                    eprintln!("({}): No such process",num);
                }
            }
        }
    }
    else if argv[0].as_str() == "bg" {
        let result;
        let num;
        let mut jid = false;
        if argv[1].find("%") != None {
            jid = true;
            result = argv[1].clone().drain(1..).collect::<String>().parse::<u32>();
        }
        else {
            result = argv[1].clone().drain(..).collect::<String>().parse::<u32>();
        }

        match result {
            Ok(val) => num = val,
            Err(_) => {
                eprintln!("{}: argument must be a PID or %jobid",argv[0]);
                return;
            },
        }
        
        let job: Option<&mut Job>;
        if jid {
            job = unsafe { JOBS.get_job_jid(num) };
        }
        else {
            job = unsafe { JOBS.get_job_pid(num as i32) };
        }

        match job {
            Some(job) => {
                job.state = ProccessState::BG;
                signal::kill(Pid::from_raw(-job.pgid),Signal::SIGCONT).unwrap();
            },
            None => {
                if jid {
                    eprintln!("%{}: No such job",num);
                }
                else {
                    eprintln!("({}): No such process",num);
                }
            }
        }
    }
}


fn waitfg(pid: i32) {
    let mut counter = 0;
    loop {
        let job = unsafe {JOBS.get_job_pid(pid)};
       
        match job {
            Some(x) => match x.state {
                ProccessState::FG => {
                    if unsafe { VERBOSE == 1} {
                        println!("{}", x);
                    }
                    if counter == 1 {
                        pause();
                    }
                }, 
                _ => break
            }
            None => break,
        }
        counter += 1;
    }

    if unsafe { VERBOSE == 1} {
        println!("Broke out");
    }
}

fn builtin_cmd(argv: &Vec<String>) -> i32 {
    if argv.len() == 0 {
        return 1;
    }
    else if argv[0].as_str() == "quit" {
        process::exit(0);
    }
    else if argv[0].as_str() == "exit" {
        process::exit(0);
    }
    else if argv[0].as_str() == "jobs" {
        unsafe {print!("{}",JOBS);}
        io::stdout().flush().unwrap();
 
        return 1;
    }
    else if argv[0].as_str() == "fg" {
        do_bgfg(&argv);
        return 1;
    }
    else if argv[0].as_str() == "bg" {
        do_bgfg(&argv);
        return 1;
    }
    
    return 0;

}


fn usage() {
    println!("Usage: shell [-hvp]");
    println!("   -h   print this message");
    println!("   -v   print additional diagnostic information");
    println!("   -p   do not emit a command prompt");
    process::exit(1);

}
