use anyhow::{Result, Context};
use std::process::{Command, Stdio, Child};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::time::SystemTime;
use libc;

#[derive(Debug)]
pub struct Job {
    pid: u32,
    command: String,
    status: JobStatus,
    start_time: SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JobStatus {
    Running,
    Stopped,
    Completed(i32),
    Failed(i32),
}

#[derive(Default)]
pub struct JobControl {
    jobs: HashMap<u32, Job>,
    last_job_id: u32,
    foreground_job: Option<u32>,
    job_mutex: Arc<Mutex<()>>,
}

impl JobControl {
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
            last_job_id: 0,
            foreground_job: None,
            job_mutex: Arc::new(Mutex::new(())),
        }
    }

    pub fn execute(&mut self, input_command: &str) -> Result<()> {
        // Check if the command contains pipes
        if input_command.contains('|') {
            // For piped commands, use the shell to execute
            let mut cmd = Command::new("sh");
            cmd.arg("-c")
               .arg(input_command)
               .stdin(Stdio::inherit())
               .stdout(Stdio::inherit())
               .stderr(Stdio::inherit());
            
            let status = cmd.status()
                .with_context(|| format!("Failed to execute command: {}", input_command))?;
            
            if !status.success() {
                eprintln!("Command failed with exit code: {}", status.code().unwrap_or(-1));
            }
            
            return Ok(());
        }
    
        // For non-piped commands, continue with the existing logic
        let parts: Vec<String> = shellwords::split(input_command)
            .with_context(|| format!("Failed to parse command: {}", input_command))?;
            
        if parts.is_empty() {
            return Ok(());
        }
    
        // Handle built-in commands
        match parts[0].as_str() {
            "jobs" => return self.list_jobs(),
            "fg" => return self.bring_to_foreground(&parts),
            "bg" => return self.continue_in_background(&parts),
            _ => {}
        }

        let background = input_command.ends_with('&');
        let exec_command = if background {
            input_command[..input_command.len()-1].trim()
        } else {
            input_command
        };

        let mut cmd = Command::new(&parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }
        
        cmd.stdin(Stdio::inherit())
           .stdout(Stdio::inherit())
           .stderr(Stdio::inherit());

        let child = cmd.spawn()
            .with_context(|| format!("Failed to spawn command: {}", exec_command))?;

        let job = Job {
            pid: child.id(),
            command: exec_command.to_string(),
            status: JobStatus::Running,
            start_time: SystemTime::now(),
        };

        self.last_job_id += 1;
        let job_id = self.last_job_id;
        self.jobs.insert(job_id, job);

        if background {
            println!("[{}] {} {}", job_id, child.id(), exec_command);
            self.monitor_background_job(job_id, child);
        } else {
            self.foreground_job = Some(job_id);
            self.wait_for_foreground_job(child)?;
        }

        Ok(())
    }

    fn monitor_background_job(&self, job_id: u32, mut child: Child) {
        let job_mutex = self.job_mutex.clone();
        std::thread::spawn(move || {
            let status = child.wait();
            let _lock = job_mutex.lock().unwrap();
            
            if let Ok(status) = status {
                if let Some(code) = status.code() {
                    if status.success() {
                        println!("[{}] Done {}", job_id, code);
                    } else {
                        println!("[{}] Exit {}", job_id, code);
                    }
                }
            }
        });
    }

    fn wait_for_foreground_job(&mut self, mut child: Child) -> Result<()> {
        let status = child.wait()
            .with_context(|| "Failed to wait for foreground process")?;

        if let Some(job_id) = self.foreground_job.take() {
            if let Some(job) = self.jobs.get_mut(&job_id) {
                job.status = if let Some(code) = status.code() {
                    if status.success() {
                        JobStatus::Completed(code)
                    } else {
                        JobStatus::Failed(code)
                    }
                } else {
                    JobStatus::Failed(-1)
                };
            }
        }

        Ok(())
    }

    pub fn list_jobs(&self) -> Result<()> {
        for (job_id, job) in &self.jobs {
            let runtime = job.start_time.elapsed()
                .unwrap_or_default()
                .as_secs();
                
            let status = match job.status {
                JobStatus::Running => "Running",
                JobStatus::Stopped => "Stopped",
                JobStatus::Completed(_) => "Done",
                JobStatus::Failed(_) => "Failed",
            };

            println!("[{}] {:?} {} ({} sec) {}", 
                job_id,
                job.pid,
                status,
                runtime,
                job.command
            );
        }
        Ok(())
    }

    pub fn bring_to_foreground(&mut self, args: &[String]) -> Result<()> {
        let job_id = if args.len() > 1 {
            args[1].parse::<u32>()
                .with_context(|| "Invalid job ID")?
        } else {
            self.last_job_id
        };

        if let Some(job) = self.jobs.get(&job_id) {
            let pid = Pid::from_raw(job.pid as i32);
            signal::kill(pid, Signal::SIGCONT)
                .with_context(|| format!("Failed to send SIGCONT to pid {}", job.pid))?;

            self.foreground_job = Some(job_id);
            println!("Brought job {} to foreground: {}", job_id, job.command);
            
            // Wait for the job to complete or stop
            self.wait_for_job(job_id)?;
        } else {
            println!("No such job: {}", job_id);
        }
        
        Ok(())
    }

    pub fn continue_in_background(&mut self, args: &[String]) -> Result<()> {
        let job_id = if args.len() > 1 {
            args[1].parse::<u32>()
                .with_context(|| "Invalid job ID")?
        } else {
            self.last_job_id
        };

        if let Some(job) = self.jobs.get(&job_id) {
            let pid = Pid::from_raw(job.pid as i32);
            signal::kill(pid, Signal::SIGCONT)
                .with_context(|| format!("Failed to send SIGCONT to pid {}", job.pid))?;

            println!("Continued job {} in background: {}", job_id, job.command);
        } else {
            println!("No such job: {}", job_id);
        }
        
        Ok(())
    }

    fn wait_for_job(&self, job_id: u32) -> Result<()> {
        if let Some(job) = self.jobs.get(&job_id) {
            let pid = Pid::from_raw(job.pid as i32);
            let mut status = 0;
            unsafe {
                libc::waitpid(pid.as_raw(), &mut status, 0);
            }
        }
        Ok(())
    }

    pub fn cleanup_completed_jobs(&mut self) {
        self.jobs.retain(|_, job| {
            matches!(job.status, JobStatus::Running | JobStatus::Stopped)
        });
    }

    pub fn handle_sigchld(&mut self) -> Result<()> {
        loop {
            match unsafe { libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) } {
                0 => break, // No more children have status changes
                -1 => break, // Error (probably no children)
                pid => {
                    if let Some(job_id) = self.find_job_by_pid(pid as u32) {
                        if let Some(job) = self.jobs.get_mut(&job_id) {
                            job.status = JobStatus::Completed(0);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn find_job_by_pid(&self, pid: u32) -> Option<u32> {
        self.jobs
            .iter()
            .find(|(_, job)| job.pid == pid)
            .map(|(job_id, _)| *job_id)
    }

    pub fn get_job_status(&self, job_id: u32) -> Option<JobStatus> {
        self.jobs.get(&job_id).map(|job| job.status.clone())
    }
}

impl Drop for JobControl {
    fn drop(&mut self) {
        // Attempt to clean up any remaining jobs
        for (_, job) in &self.jobs {
            if matches!(job.status, JobStatus::Running | JobStatus::Stopped) {
                let pid = Pid::from_raw(job.pid as i32);
                let _ = signal::kill(pid, Signal::SIGTERM);
            }
        }
    }
}