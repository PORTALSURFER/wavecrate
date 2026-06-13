//! Browser-search worker dispatch helpers.

use super::*;

impl ControllerJobs {
    /// Queue one sample-browser search job.
    pub(in super::super::super) fn send_search_job(&self, job: SearchJob) {
        self.search_job_tx.send(job);
    }
}
