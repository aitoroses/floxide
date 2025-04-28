use floxide::{node, workflow, FloxideError, Node, Transition, Workflow, WorkflowCtx};
use rllm::{
    builder::{LLMBackend, LLMBuilder},
    chat::{ChatMessage, ChatRole, MessageType},
    LLMProvider,
};
use std::sync::Arc;
use std::{
    env,
    fmt::{self, Debug},
};
use tracing::Level;

#[derive(Clone)]
pub struct LLMProviderWrapper {
    inner: Arc<Box<dyn LLMProvider>>,
}

impl LLMProviderWrapper {
    pub fn new(inner: Arc<Box<dyn LLMProvider>>) -> Self {
        Self { inner }
    }
}

impl Debug for LLMProviderWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LLMProviderWrapper")
    }
}

// --- Node 1: Generate Outline ---
node! {
    pub struct OutlineNode {
        llm: LLMProviderWrapper,
    };
    context = ();
    input = String; // Input: Article Topic
    output = String; // Output: Outline Text
    | _ctx, topic | {
        println!("OutlineNode: Generating outline for topic: '{}'", topic);
        // Call LLM to generate an outline
        let prompt = format!("Generate an outline for an article about: {}", topic);
        let messages = vec![ChatMessage { role: ChatRole::User, content: prompt, message_type: MessageType::Text }];
        let outline: String = self.llm.inner.chat(&messages).await.map_err(|e| FloxideError::Generic(e.to_string()))?.text().unwrap_or_default();
        // Pass the outline to the next step
        Ok(Transition::Next(outline))
    }
}

// --- Node 2: Draft Article ---
node! {
    pub struct DraftNode {
        llm: LLMProviderWrapper,
    };
    context = ();
    input = String; // Input: Outline Text
    output = String; // Output: Draft Article Text
    | _ctx, outline | {
        println!("DraftNode: Drafting article based on outline...");
        // Call LLM to draft the article based on the outline
        let prompt = format!("Write a detailed draft article based on the following outline:\n{}", outline);
        let messages = vec![ChatMessage { role: ChatRole::User, content: prompt, message_type: MessageType::Text }];
        let draft: String = self.llm.inner.chat(&messages).await.map_err(|e| FloxideError::Generic(e.to_string()))?.text().unwrap_or_default();
        // Pass the draft to the next step
        Ok(Transition::Next(draft))
    }
}

// --- Node 3: Review Article ---
node! {
    pub struct ReviewNode {
        llm: LLMProviderWrapper,
    };
    context = ();
    input = String; // Input: Draft Article Text
    output = String; // Output: Final Article Text
    | _ctx, draft | {
        println!("ReviewNode: Reviewing and finalizing draft...");
        // Call LLM to review and finalize the draft
        let prompt = format!("Review and finalize the following article draft. Provide the final polished version without any additional text:\n{}", draft);
        let messages = vec![ChatMessage { role: ChatRole::User, content: prompt, message_type: MessageType::Text }];
        let final_article: String = self.llm.inner.chat(&messages).await.map_err(|e| FloxideError::Generic(e.to_string()))?.text().unwrap_or_default();
        // Pass the final article as the workflow result
        Ok(Transition::Next(final_article))
    }
}

// --- Workflow Definition: Connecting the nodes ---
workflow! {
    pub struct ArticleWriterWorkflow {
        outline: OutlineNode,
        draft: DraftNode,
        review: ReviewNode,
    }
    start = outline; // Start with the outline node
    context = ();
    edges {
        // Define the sequence: outline -> draft -> review
        outline => { [draft] };
        draft => { [review] };
        review => {}; // review is the final node
    }
}

#[tokio::main]
async fn main() -> Result<(), FloxideError> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    // Initialize the LLM for chat completion
    let llm = LLMBuilder::new()
        .backend(LLMBackend::OpenAI)
        .api_key(env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set"))
        .model("gpt-4o")
        .temperature(0.7)
        .build()
        .expect("Failed to build LLM");

    let llm = LLMProviderWrapper::new(Arc::new(llm));

    let workflow = ArticleWriterWorkflow {
        outline: OutlineNode { llm: llm.clone() },
        draft: DraftNode { llm: llm.clone() },
        review: ReviewNode { llm: llm.clone() },
    };
    let ctx = WorkflowCtx::new(());
    let result = workflow
        .run(&ctx, "Rust Programming Language".to_string())
        .await?;

    println!("Generated article: {}", result);

    Ok(())
}
