use crate::model::CommitId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphRow {
    pub text: String,
}

#[derive(Default)]
pub struct Graph {
    columns: Vec<CommitId>,
}

impl Graph {
    pub fn next_row(&mut self, id: &CommitId, parents: &[CommitId]) -> GraphRow {
        let my_col = match self.columns.iter().position(|c| c == id) {
            Some(pos) => pos,
            None => {
                self.columns.push(id.clone());
                self.columns.len() - 1
            }
        };

        let converging: Vec<usize> = self
            .columns
            .iter()
            .enumerate()
            .filter(|(idx, candidate)| *idx != my_col && *candidate == id)
            .map(|(idx, _)| idx)
            .collect();

        let mut text = String::with_capacity(self.columns.len() * 2);
        for idx in 0..self.columns.len() {
            text.push(if idx == my_col { '*' } else { '|' });
            text.push(' ');
        }

        for &idx in converging.iter().rev() {
            self.columns.remove(idx);
        }
        let adjusted = my_col - converging.iter().filter(|&&idx| idx < my_col).count();

        match parents {
            [] => {
                self.columns.remove(adjusted);
            }
            [first] => {
                self.columns[adjusted] = first.clone();
            }
            [first, rest @ ..] => {
                self.columns[adjusted] = first.clone();
                let mut insert_at = adjusted + 1;
                for parent in rest {
                    if !self.columns.contains(parent) {
                        self.columns.insert(insert_at, parent.clone());
                        insert_at += 1;
                    }
                }
            }
        }

        GraphRow { text }
    }
}
