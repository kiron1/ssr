pub struct Query {}


impl Query {
    pub fn match(&self) -> Result {
        
        let mut qcursor = tree_sitter::QueryCursor::new();
        let matches = qcursor.matches(&query, tree.root_node(), content.as_slice());
        for m in matches {
            let id = m.id();
            let pi = m.pattern_index;
            for capture in m.captures {
                let start = capture.node.start_position();
                let end = capture.node.end_position();
                let i = capture.index;
                let capture_name = query.capture_names()[capture.index as usize];
                let capture = capture.node.utf8_text(content.as_slice())?;
                println!("{id} {start} {end} {i} {pi} {capture_name} {capture}");
            }
            // let capture m.nodes_for_capture_index();
        }
    }
 }
