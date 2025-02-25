use async_trait::async_trait;
use flowrs_core::{
    DefaultAction,
    batch::BatchContext,
    error::FlowrsError,
    Node, NodeId, NodeOutcome,
};
use rand::Rng;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;
use tracing_subscriber::fmt;
use uuid::Uuid;

/// Represents an image with various properties
#[derive(Debug, Clone)]
struct Image {
    id: String,
    name: String,
    width: u32,
    height: u32,
    format: String,
    data: String,
    processing_time_ms: u64,
}

impl Image {
    fn new(name: &str, width: u32, height: u32, format: &str) -> Self {
        // Simulate random processing time for this image
        let mut rng = rand::thread_rng();
        let processing_time_ms = rng.gen_range(100..1000);
        
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            width,
            height,
            format: format.to_string(),
            data: format!("Original image data for {}", name),
            processing_time_ms,
        }
    }
    
    async fn resize(&self, new_width: u32, new_height: u32) -> Result<Image, FlowrsError> {
        // Simulate the time it takes to resize an image
        sleep(Duration::from_millis(self.processing_time_ms)).await;
        
        let mut resized = self.clone();
        resized.width = new_width;
        resized.height = new_height;
        resized.data = format!("Resized: {}", self.data);
        info!("Resized image {} to {}x{}", self.name, new_width, new_height);
        
        Ok(resized)
    }
    
    async fn convert_format(&self, new_format: &str) -> Result<Image, FlowrsError> {
        // Simulate the time it takes to convert an image
        sleep(Duration::from_millis(self.processing_time_ms / 2)).await;
        
        let mut converted = self.clone();
        converted.format = new_format.to_string();
        converted.data = format!("Converted to {}: {}", new_format, self.data);
        info!("Converted image {} to {}", self.name, new_format);
        
        Ok(converted)
    }
}

/// Context for batch processing of images
#[derive(Debug, Clone)]
struct ImageBatchContext {
    images: Vec<Image>,
    current_image: Option<Image>,
    processed_count: usize,
    failed_count: usize,
    target_format: String,
    stats: HashMap<String, usize>,
}

impl ImageBatchContext {
    fn new(images: Vec<Image>, target_format: &str) -> Self {
        Self {
            images,
            current_image: None,
            processed_count: 0,
            failed_count: 0,
            target_format: target_format.to_string(),
            stats: HashMap::new(),
        }
    }
    
    fn add_stat(&mut self, key: &str) {
        *self.stats.entry(key.to_string()).or_insert(0) += 1;
    }
    
    fn print_stats(&self) {
        info!("Batch processing statistics:");
        for (key, value) in &self.stats {
            info!("  {}: {}", key, value);
        }
    }
}

impl BatchContext<Image> for ImageBatchContext {
    fn get_batch_items(&self) -> Result<Vec<Image>, FlowrsError> {
        Ok(self.images.clone())
    }
    
    fn create_item_context(&self, item: Image) -> Result<Self, FlowrsError> {
        // Create a new context for a single item
        let mut ctx = self.clone();
        ctx.images = Vec::new();
        ctx.current_image = Some(item);
        Ok(ctx)
    }
    
    fn update_with_results(
        &mut self,
        results: &[Result<Image, FlowrsError>],
    ) -> Result<(), FlowrsError> {
        // Update statistics
        self.processed_count = results.iter().filter(|r| r.is_ok()).count();
        self.failed_count = results.iter().filter(|r| r.is_err()).count();
        
        // Update statistics
        for result in results {
            match result {
                Ok(_) => self.add_stat("success"),
                Err(_) => self.add_stat("failure"),
            }
        }
        
        Ok(())
    }
}

struct SimpleImageProcessor {
    id: NodeId,
}

impl SimpleImageProcessor {
    fn new(id: &str) -> Self {
        SimpleImageProcessor { id: id.to_string() }
    }
}

#[async_trait]
impl Node<ImageBatchContext, DefaultAction> for SimpleImageProcessor {
    type Output = Image;

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    async fn process(
        &self,
        ctx: &mut ImageBatchContext,
    ) -> Result<NodeOutcome<Self::Output, DefaultAction>, FlowrsError> {
        // Get the image from the context
        match ctx.current_image.as_ref() {
            Some(image) => {
                // Simulate image processing operations
                let mut rng = rand::thread_rng();
                if rng.gen_bool(0.15) {
                    // 15% chance of failure
                    return Err(FlowrsError::node_execution(
                        self.id(),
                        "Random failure during image processing",
                    ));
                }

                // Simulate processing (resize and format conversion)
                let new_width = (image.width as f64 * 0.8) as u32;
                let new_height = (image.height as f64 * 0.8) as u32;
                let processed = Image {
                    id: image.id.clone(),
                    name: image.name.clone(),
                    width: new_width,
                    height: new_height,
                    format: "webp".to_string(),
                    data: format!("Processed: {}", image.data),
                    processing_time_ms: image.processing_time_ms,
                };

                // Log and return result
                tracing::info!(
                    node_id = %self.id(),
                    "Processed image {}. New dimensions: {}x{}, format: webp",
                    image.id, new_width, new_height
                );

                Ok(NodeOutcome::Success(processed))
            }
            None => Err(FlowrsError::node_execution(self.id(), "No image found in context")),
        }
    }
}

// Process a single image using the SimpleImageProcessor
async fn process_image(image: Image) -> Result<Image, FlowrsError> {
    // Create a context with just this image
    let mut ctx = ImageBatchContext {
        images: Vec::new(),
        current_image: Some(image),
        processed_count: 0,
        failed_count: 0,
        target_format: "webp".to_string(),
        stats: HashMap::new(),
    };
    
    // Process the image
    let processor = SimpleImageProcessor::new("image_processor");
    match processor.process(&mut ctx).await {
        Ok(NodeOutcome::Success(image)) => Ok(image),
        Ok(_) => Err(FlowrsError::node_execution(
            processor.id(), 
            "Unexpected outcome from processor"
        )),
        Err(e) => Err(e),
    }
}

// Process images in parallel with a given parallelism limit
async fn process_batch(
    images: Vec<Image>, 
    parallelism: usize
) -> Vec<Result<Image, FlowrsError>> {
    use tokio::sync::Semaphore;
    use futures::stream::{self, StreamExt};
    
    let semaphore = std::sync::Arc::new(Semaphore::new(parallelism));
    
    let tasks = stream::iter(images)
        .map(|image| {
            let semaphore = semaphore.clone();
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                let result = process_image(image).await;
                drop(_permit);
                result
            }
        })
        .buffer_unordered(parallelism)
        .collect::<Vec<_>>()
        .await;
    
    tasks
}

fn main() {
    // Initialize tracing
    fmt::init();

    // Create a batch of images to process
    let mut images = Vec::new();
    let mut rng = rand::thread_rng();
    for i in 1..10 {
        images.push(Image {
            id: format!("img_{}", i),
            name: format!("Image {}", i),
            width: 1920,
            height: 1080,
            format: "jpg".to_string(),
            data: format!("Original image data {}", i),
            processing_time_ms: rng.gen_range(100..500),
        });
    }

    // Set up runtime
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    // Execute the batch processing
    let results = rt.block_on(async {
        process_batch(images, 4).await
    });
    
    // Print the results
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let error_count = results.len() - success_count;
    
    println!("Batch processing completed!");
    println!("Total items: {}", results.len());
    println!("Successfully processed: {}", success_count);
    println!("Failed: {}", error_count);
    
    // Print individual results
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(image) => println!("Item {}: Successfully processed to {}x{} {}", 
                                 i+1, image.width, image.height, image.format),
            Err(e) => println!("Item {}: Failed with error: {}", i+1, e),
        }
    }
} 