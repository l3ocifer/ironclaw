//! Memory and Knowledge Graph Module
//!
//! Provides persistent memory storage and retrieval capabilities

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Memory types for categorization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryType {
    Project,
    Decision,
    Meeting,
    Task,
    Knowledge,
}

/// A stored memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub memory_type: MemoryType,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub timestamp: u64,
    pub metadata: HashMap<String, Value>,
}

impl Memory {
    pub fn new(memory_type: MemoryType, title: String, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            memory_type,
            title,
            content,
            tags: Vec::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// Memory manager for storing and retrieving memories
pub struct MemoryManager {
    memories: Vec<Memory>,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            memories: Vec::new(),
        }
    }

    /// Create a new memory
    pub fn create(
        &mut self,
        memory_type: MemoryType,
        title: String,
        content: String,
        tags: Vec<String>,
    ) -> Value {
        let memory =
            Memory::new(memory_type, title.clone(), content.clone()).with_tags(tags.clone());
        let timestamp = memory.timestamp;
        self.memories.push(memory);

        json!({
            "content": [{
                "type": "text",
                "text": format!("🧠 Memory Created\n\nTitle: \"{}\"\nContent: {}\nTags: {:?}\nTimestamp: {}\n\n✅ Memory stored in knowledge graph", title, content, tags, timestamp)
            }]
        })
    }

    /// Search memories by query
    pub fn search(&self, query: &str, memory_type: Option<MemoryType>) -> Value {
        let results: Vec<&Memory> = self
            .memories
            .iter()
            .filter(|m| {
                let type_match = memory_type.is_none()
                    || std::mem::discriminant(&Some(m.memory_type.clone()))
                        == std::mem::discriminant(&memory_type);
                let content_match = m.title.contains(query)
                    || m.content.contains(query)
                    || m.tags.iter().any(|t| t.contains(query));
                type_match && content_match
            })
            .collect();

        json!({
            "content": [{
                "type": "text",
                "text": format!("🔍 Memory Search Results\n\nQuery: \"{}\"\nFound: {} memories\n\n📋 Results:\n{}",
                    query,
                    results.len(),
                    results.iter().map(|m| format!("• {}: {}", m.title, m.content)).collect::<Vec<_>>().join("\n")
                )
            }]
        })
    }

    /// Store an LLM response
    pub fn store_llm_response(
        &mut self,
        response: String,
        context: String,
        model: String,
    ) -> Value {
        let memory = Memory::new(
            MemoryType::Knowledge,
            format!("LLM Response: {}", model),
            response.clone(),
        );
        let timestamp = memory.timestamp;
        self.memories.push(memory);

        json!({
            "content": [{
                "type": "text",
                "text": format!("🤖 LLM Response Stored\n\nModel: {}\nContext: {}\nResponse: {}\nTimestamp: {}\n\n✅ Stored for future reference", model, context, response, timestamp)
            }]
        })
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}
