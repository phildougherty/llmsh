use nix::sys::signal::{self, Signal, SigHandler, SigAction, SigSet, SaFlags};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use log::debug;

// Global flag to indicate if Ctrl+C was pressed
lazy_static::lazy_static! {
    pub static ref INTERRUPT_RECEIVED: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

pub struct SignalHandler;

impl SignalHandler {
    pub fn initialize() -> Result<(), nix::Error> {
        debug!("Initializing signal handlers");
        
        // Set up SIGINT (Ctrl+C) handler
        let sigint_action = SigAction::new(
            SigHandler::Handler(Self::handle_sigint),
            SaFlags::empty(),
            SigSet::empty(),
        );
        unsafe { signal::sigaction(Signal::SIGINT, &sigint_action)? };
        
        // Set up SIGTSTP (Ctrl+Z) handler
        let sigtstp_action = SigAction::new(
            SigHandler::Handler(Self::handle_sigtstp),
            SaFlags::empty(),
            SigSet::empty(),
        );
        unsafe { signal::sigaction(Signal::SIGTSTP, &sigtstp_action)? };
        
        // Set up SIGCHLD handler for child process termination
        let sigchld_action = SigAction::new(
            SigHandler::Handler(Self::handle_sigchld),
            SaFlags::empty(),
            SigSet::empty(),
        );
        unsafe { signal::sigaction(Signal::SIGCHLD, &sigchld_action)? };
        
        Ok(())
    }
    
    extern "C" fn handle_sigint(_: i32) {
        INTERRUPT_RECEIVED.store(true, Ordering::SeqCst);
        // Print a newline to ensure the next prompt appears on a fresh line
        println!();
    }
    
    extern "C" fn handle_sigtstp(_: i32) {
        // Default behavior is fine for now - just let the process be suspended
    }
    
    extern "C" fn handle_sigchld(_: i32) {
        // This will be handled by the job control system
        // We just need to catch the signal to prevent the default behavior
    }
    
    pub fn was_interrupted() -> bool {
        let was_interrupted = INTERRUPT_RECEIVED.load(Ordering::SeqCst);
        if was_interrupted {
            INTERRUPT_RECEIVED.store(false, Ordering::SeqCst);
        }
        was_interrupted
    }
}
