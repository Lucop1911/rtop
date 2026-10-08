use std::cmp::Ordering;

use crate::{App, DisplayRow, SortColumn};

impl App {
    /// Rebuilds the flat process list.
    ///
    /// Filters the flat process list and sorts it globally by the current
    /// sort column — one single list with no nesting, the way htop/top
    /// display processes. Runs on every sample and on every
    /// filter/sort/search change; touches no `/proc` files.
    pub fn rebuild_display(&mut self) {
        // Remember which process was selected so the cursor can follow it
        // (htop's "cursor follows process") instead of sticking to a row
        // index that means something different after re-sorting.
        let prev_selected_pid = self
            .table_state
            .selected()
            .and_then(|i| self.display.get(i))
            .map(|row| self.processes[row.proc_idx].pid);

        self.pid_index.clear();
        for (idx, info) in self.processes.iter().enumerate() {
            self.pid_index.insert(info.pid, idx);
        }

        // One global sort over all matching processes: every row is
        // compared with the same comparator, so a busy child sits at the
        // top next to unrelated processes, exactly like htop.
        let mut indices: Vec<usize> = (0..self.processes.len())
            .filter(|&idx| self.process_matches_filters(&self.processes[idx]))
            .collect();

        let processes: &[crate::ProcessInfo] = &self.processes;
        let sort_column = self.sort_column;
        let reverse = self.reverse_sort;
        let cmp = move |a: &usize, b: &usize| {
            let (x, y) = (&processes[*a], &processes[*b]);
            let ordering = match sort_column {
                SortColumn::Pid => x.pid.cmp(&y.pid),
                SortColumn::Name => x.name.cmp(&y.name),
                SortColumn::Command => x.command.cmp(&y.command),
                SortColumn::Cpu => x
                    .cpu_usage
                    .partial_cmp(&y.cpu_usage)
                    .unwrap_or(Ordering::Equal),
                SortColumn::Memory => x.memory.cmp(&y.memory),
            };
            if reverse {
                ordering.reverse()
            } else {
                ordering
            }
        };
        indices.sort_by(&cmp);

        self.display = indices
            .into_iter()
            .map(|proc_idx| DisplayRow { proc_idx })
            .collect();

        // Keep the selection inside the new list.
        let len = self.display.len();
        if len == 0 {
            self.table_state.select(None);
            self.viewport_offset = 0;
        } else {
            // Follow the previously selected process if it is still in the
            // view; otherwise clamp the old index (process exited or was
            // filtered out).
            let selected = prev_selected_pid
                .and_then(|pid| {
                    self.display
                        .iter()
                        .position(|row| self.processes[row.proc_idx].pid == pid)
                })
                .unwrap_or_else(|| self.table_state.selected().unwrap_or(0).min(len - 1));
            self.table_state.select(Some(selected));
            self.ensure_visible(selected);
            self.viewport_offset = self.viewport_offset.min(len - 1);
        }
    }

    fn process_matches_filters(&self, info: &crate::ProcessInfo) -> bool {
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            if !info.name.to_lowercase().contains(&query)
                && !info.pid.to_string().contains(&self.search_query)
            {
                return false;
            }
        }

        if let Some(ref user_filter) = self.user_filter {
            let query = user_filter.to_lowercase();
            let by_name = info
                .user_id
                .and_then(|uid| self.user_names.get(&uid))
                .map(|name| name.to_lowercase().contains(&query))
                .unwrap_or(false);
            let by_id = info
                .user_id
                .map(|uid| uid.to_string().contains(user_filter))
                .unwrap_or(false);
            if !by_name && !by_id {
                return false;
            }
        }

        if let Some(ref status_filter) = self.status_filter
            && !info
                .status
                .to_lowercase()
                .contains(&status_filter.to_lowercase())
        {
            return false;
        }

        if let Some(threshold) = self.cpu_threshold
            && info.cpu_usage < threshold
        {
            return false;
        }

        if let Some(threshold) = self.memory_threshold
            && info.memory < threshold
        {
            return false;
        }

        true
    }
}
