use crate::refiner;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use threadpool::ThreadPool;

/**
A StringFuture can perform diffing in a background thread.

Doing get() on a future that isn't done yet will block until the result is
available.
*/
pub(crate) struct StringFuture {
    // This field is only valid if we're done with the result_receiver (next
    // field)
    result: String,

    // If available, get() will await a result on this receiver, then populate
    // the result field and return it
    result_receiver: Option<Receiver<String>>,
}

impl StringFuture {
    /// Create an already-finished future
    pub fn from_string(result: String) -> StringFuture {
        return StringFuture {
            result,
            result_receiver: None,
        };
    }

    /// Call get() to get the result of this diff
    pub fn from_oldnew(
        old_text: String,
        new_text: String,
        thread_pool: &ThreadPool,
    ) -> StringFuture {
        // Create a String channel
        let (sender, receiver): (SyncSender<String>, Receiver<String>) = sync_channel(1);

        // Start diffing in a thread
        thread_pool.execute(move || {
            let mut result = String::new();
            for line in refiner::format(&old_text, &new_text) {
                result.push_str(&line);
                result.push('\n');
            }

            // Done, channel the result!
            sender.send(result).unwrap();
        });

        return StringFuture {
            result: "".to_string(),
            result_receiver: Some(receiver),
        };
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn is_empty(&mut self) -> bool {
        return self.get().is_empty();
    }

    pub fn get(&mut self) -> &str {
        // If the result is still pending...
        if let Some(receiver) = &self.result_receiver {
            // ... wait for it
            self.result = receiver.recv().unwrap();
            self.result_receiver = None;
        }

        return &self.result;
    }
}
