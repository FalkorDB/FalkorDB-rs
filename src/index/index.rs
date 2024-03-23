extern crate tantivy;

use tantivy::schema::Schema;
use tantivy::{Document, Index, IndexReader, IndexWriter, ReloadPolicy};

pub struct IndexWrapper {
    index: Index,
    writer: IndexWriter,
    reader: IndexReader,
}

#[derive(Debug, Clone)]

pub struct IndexError {
    message: String,
}

// From tantivy::TantivyError to IndexError
impl From<tantivy::TantivyError> for IndexError {
    fn from(error: tantivy::TantivyError) -> Self {
        IndexError {
            message: error.to_string(),
        }
    }
}

impl IndexWrapper {
    pub fn new(schema: Schema) -> Result<IndexWrapper, IndexError> {
        let index = Index::create_in_ram(schema);
        let writer = index.writer(50_000_000)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;

        Ok(IndexWrapper {
            index,
            writer,
            reader,
        })
    }

    pub fn add_document(
        &mut self,
        doc: Document,
    ) {
        self.writer.add_document(doc);
    }

    // pub fn search(&self, query: Query) -> Result<Vec<Document>, IndexError> {
    //     let searcher = self.reader.searcher();
    //     let res = searcher.search(&query, &TopDocs::with_limit(10))?.
    //         iter().
    //         map(|(_, doc_address)| searcher.doc(doc_address).unwrap()).
    //         collect();

    //     Ok(res)
    // }

    pub fn commit(&mut self) {
        // self.index.commit();
        // self.index.
    }
}
