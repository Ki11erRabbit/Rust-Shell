
use std::fmt;

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
    pub pid: i32,
    pub pgid: i32,
    pub jid: u32,
    pub state: ProccessState,
    pub cmdline: String,

}

impl Job {
    pub fn new(pid: i32, pgid: i32, jid: u32, state: ProccessState, cmdline: &str) -> Self {
        Self {pid: pid, pgid: pgid, jid: jid, state: state, cmdline: cmdline.to_string()}
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

    pub fn addjob(&mut self, pid: i32, pgid: i32, state: ProccessState, cmdline: &str) {
       self.jobs.push(Job::new(pid,pgid,self.next_jid,state,cmdline)); 
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
