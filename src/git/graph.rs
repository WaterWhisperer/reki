use git2::Oid;

/// ASCII commit-graph lane tracker.
///
/// Call [`Graph::next_row`] for each commit in time order; the returned string
/// contains `*`, `|` and spaces that the UI can colorize per character.
pub struct Graph {
    /// Active lanes, each heading towards a target OID.
    columns: Vec<Oid>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
        }
    }

    /// Produce the graph string for one commit row (e.g. `"* | "`).
    pub fn next_row(&mut self, id: Oid, parents: &[Oid]) -> String {
        // Find (or allocate) the column for this commit.
        let my_col = match self.columns.iter().position(|&c| c == id) {
            Some(pos) => pos,
            None => {
                self.columns.push(id);
                self.columns.len() - 1
            }
        };

        // Detect converging lanes (other columns also heading to this commit).
        let converging: Vec<usize> = self
            .columns
            .iter()
            .enumerate()
            .filter(|&(i, &c)| i != my_col && c == id)
            .map(|(i, _)| i)
            .collect();

        // Build node line: `*` at my_col, `|` elsewhere, space-separated.
        let num_cols = self.columns.len();
        let mut line = String::with_capacity(num_cols * 2);
        for i in 0..num_cols {
            line.push(if i == my_col { '*' } else { '|' });
            line.push(' ');
        }

        // Remove converging columns (right-to-left to keep indices stable).
        for &i in converging.iter().rev() {
            self.columns.remove(i);
        }
        let adjusted = my_col - converging.iter().filter(|&&i| i < my_col).count();

        // Update lanes based on parent count.
        match parents.len() {
            0 => {
                self.columns.remove(adjusted);
            }
            1 => {
                self.columns[adjusted] = parents[0];
            }
            _ => {
                // Merge: first parent keeps the lane, extras spawn new lanes.
                self.columns[adjusted] = parents[0];
                let mut ins = adjusted + 1;
                for &p in &parents[1..] {
                    if !self.columns.contains(&p) {
                        self.columns.insert(ins, p);
                        ins += 1;
                    }
                }
            }
        }

        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(b: u8) -> Oid {
        let mut bytes = [0u8; 20];
        bytes[0] = b;
        Oid::from_bytes(&bytes).unwrap()
    }

    #[test]
    fn linear_history() {
        let mut g = Graph::new();
        assert_eq!(g.next_row(oid(1), &[oid(2)]), "* ");
        assert_eq!(g.next_row(oid(2), &[oid(3)]), "* ");
        assert_eq!(g.next_row(oid(3), &[]), "* ");
    }

    #[test]
    fn branch_and_merge() {
        let mut g = Graph::new();
        assert_eq!(g.next_row(oid(1), &[oid(2), oid(3)]), "* ");
        assert_eq!(g.next_row(oid(2), &[oid(4)]), "* | ");
        assert_eq!(g.next_row(oid(3), &[oid(4)]), "| * ");
        assert_eq!(g.next_row(oid(4), &[oid(5)]), "* | ");
        assert_eq!(g.next_row(oid(5), &[]), "* ");
    }
}
