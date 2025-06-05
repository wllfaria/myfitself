use derive_more::{Display, Error, From};
use food_aggregator::models::foods::Foods;
use serde::Serialize;
use sqlx::PgConnection;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, STORED, Schema, SchemaBuilder, TEXT, Value};
use tantivy::{Index, IndexReader, ReloadPolicy, TantivyDocument, doc};

const INDEX_MEMORY_BUDGET: usize = 50_000_000; // 50MB

type Result<T, E = SearchError> = std::result::Result<T, E>;

#[derive(Debug, Display, From, Error)]
pub enum SearchError {
    #[from]
    Database(sqlx::Error),
    #[from]
    Tantivy(tantivy::error::TantivyError),
}

#[derive(Clone)]
pub struct SearchService {
    index: Index,
    schema: Schema,
    reader: IndexReader,
    id_field: Field,
    name_field: Field,
    source_field: Field,
}

#[derive(Debug, Serialize)]
pub struct FoodSearchResult {
    id: String,
    name: String,
    source: String,
}

impl SearchService {
    pub async fn new(executor: &mut PgConnection) -> Result<SearchService, SearchError> {
        let schema = build_schema();
        let index = build_index(executor, schema.clone()).await?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let id_field = schema.get_field("id")?;
        let name_field = schema.get_field("name")?;
        let source_field = schema.get_field("source")?;

        Ok(SearchService {
            schema,
            index,
            reader,
            id_field,
            name_field,
            source_field,
        })
    }

    pub fn search<S: AsRef<str>>(
        &self,
        query: S,
        limit: usize,
    ) -> Result<Vec<FoodSearchResult>, SearchError> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.name_field]);
        let query = query_parser.parse_query(query.as_ref()).unwrap();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (_, doc_addr) in top_docs {
            let document: TantivyDocument = searcher.doc(doc_addr)?;

            let id = document
                .get_first(self.id_field)
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned)
                .expect("document id must be a string");

            let name = document
                .get_first(self.name_field)
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned)
                .expect("document name must be a string");

            let source = document
                .get_first(self.source_field)
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned)
                .expect("document source must be a string");

            results.push(FoodSearchResult { id, name, source });
        }

        Ok(results)
    }
}

fn build_schema() -> Schema {
    let mut schema_builder = SchemaBuilder::new();
    schema_builder.add_text_field("id", TEXT | STORED);
    schema_builder.add_text_field("name", TEXT | STORED);
    schema_builder.add_text_field("source", TEXT | STORED);
    schema_builder.build()
}

async fn build_index(executor: &mut PgConnection, schema: Schema) -> Result<Index, SearchError> {
    let foods = Foods::get_for_search(executor).await?;

    let id = schema.get_field("id").expect("schema must have field id");
    let name = schema
        .get_field("name")
        .expect("schema must have field name");
    let source = schema
        .get_field("source")
        .expect("schema must have field source");

    let index = Index::create_in_ram(schema);
    let mut index_writer = index.writer(INDEX_MEMORY_BUDGET)?;

    for food in foods {
        index_writer.add_document(doc!(
            id => food.id().to_string(),
            name => food.name(),
            source => food.source(),
        ))?;
    }

    index_writer.commit()?;

    Ok(index)
}
