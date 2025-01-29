use std::{
    env, error, fmt, io,
    process::{self, Command, Stdio},
};
use stdio_override::StdoutOverride;

const PAGER_FORKBOMB_STOP: &str = "_PAGER_FORKBOMB_STOP";

pub(crate) struct Pager {
    pager_env: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Error {
    message: String,
    source: io::Error,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "{}: {}", self.message, self.source);
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        return Some(&self.source);
    }
}

impl Pager {
    pub fn new() -> Self {
        return Pager { pager_env: None };
    }

    pub fn with_custom_pager_env_var(&mut self, pager_env: &str) -> Self {
        return Pager {
            pager_env: Some(pager_env.to_string()),
        };
    }

    fn try_page_stdout(
        &mut self,
        pager_space_separated: &str,
    ) -> Result<process::Child, io::Error> {
        let pager_cmdline: Vec<&str> = pager_space_separated.split_whitespace().collect();
        let mut command = Command::new(pager_cmdline[0]);
        for arg in pager_cmdline.iter().skip(1) {
            command.arg(arg);
        }

        if env::var(PAGER_FORKBOMB_STOP).is_ok() {
            // Try preventing fork bombing if $PAGER is set to riff
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Already paging, {} is set", PAGER_FORKBOMB_STOP),
            ));
        }
        command.env(PAGER_FORKBOMB_STOP, "1");

        if env::var("LESS").is_err() {
            // Set by git when paging
            command.env("LESS", "FRX");
        }

        // FIXME: Should we set similar variables for moar that git sets for less?
        //  That would be MOAR="--quit-if-one-screen --no-clear-on-exit"

        if env::var("LV").is_err() {
            // Set by git when paging
            command.env("LV", "-c");
        }

        command.stdin(Stdio::piped());
        return command.spawn();
    }

    fn page_to_process(mut pager: process::Child, f: impl FnOnce()) -> Result<(), Error> {
        // If this unwrap() fails, there's probably something wrong with
        // try_page_stdout(). It should ensure we can access the pager's
        // stdin.
        let pager_stdin = pager.stdin.take().unwrap();

        // Start capturing stdout
        let stdout_capture = StdoutOverride::from_io(pager_stdin);
        if let Err(e) = stdout_capture {
            return Err(Error {
                message: "Failed to override stdout".to_string(),
                source: e,
            });
        }

        // Call the function that will write to stdout
        f();

        // Stop capturing stdout
        drop(stdout_capture);

        // Wait for the pager to finish
        let wait_result = pager.wait();
        if let Err(e) = wait_result {
            return Err(Error {
                message: "Failed to wait for pager".to_string(),
                source: e,
            });
        }

        // Handle pager unexpected exit
        let exit_status = wait_result.unwrap();
        if !exit_status.success() {
            // NOTE: Maybe we should have captured the pager's stderr and
            // included it in the error message?
            return Err(Error {
                message: "Pager failed".to_string(),
                source: io::Error::new(io::ErrorKind::Other, format!("{}", exit_status)),
            });
        }

        // Paging succeessful, all done
        return Ok(());
    }

    pub fn page_stdout(&mut self, f: impl FnOnce()) -> Result<(), Error> {
        if let Some(pager_env_var) = &self.pager_env.take() {
            // Custom pager environment variable name set by developer
            if let Ok(pager_env) = env::var(pager_env_var) {
                // Custom pager environment variable set by user
                match self.try_page_stdout(&pager_env) {
                    Ok(pager) => return Self::page_to_process(pager, f),

                    // User explicitly set the custom PAGER variable. If that
                    // wasn't launchable, that's a failure.
                    Err(e) => {
                        return Err(Error {
                            message: format!(
                                "Failed to page with ${}='{}'",
                                pager_env_var, pager_env
                            ),
                            source: e,
                        })
                    }
                }
            }
        }

        if let Ok(pager_env) = env::var("PAGER") {
            match self.try_page_stdout(&pager_env) {
                Ok(pager) => return Self::page_to_process(pager, f),

                // User explicitly set $PAGER. If that doesn't exist, that's a failure.
                Err(e) => {
                    return Err(Error {
                        message: format!("Failed to page with $PAGER='{}'", pager_env),
                        source: e,
                    })
                }
            }
        }

        if let Ok(pager) = self.try_page_stdout("moar") {
            return Self::page_to_process(pager, f);
        }

        if let Ok(pager) = self.try_page_stdout("less") {
            return Self::page_to_process(pager, f);
        }

        // No pager found, just do what git does and print to stdout:
        // https://github.com/git/git/blob/5f8f7081f7761acdf83d0a4c6819fe3d724f01d7/pager.c#L144-L150
        f();

        return Ok(());
    }
}
