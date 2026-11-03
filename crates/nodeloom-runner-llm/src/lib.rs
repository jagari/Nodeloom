//! Built-in plugin that answers prompts with a local LLM through Ollama's
//! HTTP API on this machine (`127.0.0.1:11434`). The master sends prompts as
//! data; nothing here runs a command (AGENTS.md rule 1).

use std::time::Duration;

pub mod server;

use async_trait::async_trait;
use nodeloom_core::{Runner, RunnerError, validate::validate_model_name};
use nodeloom_proto::{RunnerKind, TaskPayload};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Where Ollama listens by default. Only this machine's own server is used.
pub const DEFAULT_URL: &str = "http://127.0.0.1:11434";

/// Answer length when the job does not say, so a demo stays quick and a
/// dashboard stays readable; Ollama's `num_predict` counts tokens.
const DEFAULT_ANSWER_TOKENS: u32 = 200;
/// Context window asked of Ollama. Its default (2048 to 4096 tokens,
/// depending on the version) silently cuts the start of a long prompt, such
/// as a document part, and the model then summarizes half of it. Kept the
/// same for every prompt so a model is not reloaded between jobs.
const CONTEXT_TOKENS: u32 = 8192;
/// The longest answer sent back, in characters.
const MAX_ANSWER_CHARS: usize = 4000;
/// A small model on a CPU answers a short prompt in seconds; a cold start
/// (loading the model into memory) can take much longer.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(300);
const DETECT_TIMEOUT: Duration = Duration::from_secs(2);

pub struct LlmRunner {
    base: String,
    client: reqwest::Client,
    models: Vec<String>,
}

#[derive(Deserialize)]
struct Tags {
    #[serde(default)]
    models: Vec<Tag>,
}

#[derive(Deserialize)]
struct Tag {
    name: String,
}

#[derive(Deserialize)]
struct Generated {
    #[serde(default)]
    response: String,
}

#[derive(Deserialize)]
struct OllamaError {
    error: String,
}

impl LlmRunner {
    /// Asks the Ollama server at `base` which models it has. `None` when
    /// nothing answers or no model is installed: then this machine simply
    /// does not offer LLM work.
    pub async fn detect(base: &str) -> Option<Self> {
        let client = reqwest::Client::new();
        let reply = client
            .get(format!("{base}/api/tags"))
            .timeout(DETECT_TIMEOUT)
            .send()
            .await
            .ok()?;
        let tags: Tags = reply.error_for_status().ok()?.json().await.ok()?;
        let mut models: Vec<String> = tags
            .models
            .into_iter()
            .map(|t| t.name)
            .filter(|name| validate_model_name(name).is_ok())
            .collect();
        models.sort();
        models.dedup();
        if models.is_empty() {
            warn!("Ollama answers at {base} but has no models; run `ollama pull <model>`");
            return None;
        }
        info!("local LLM via Ollama at {base}: {}", models.join(", "));
        Some(Self {
            base: base.trim_end_matches('/').to_owned(),
            client,
            models,
        })
    }

    async fn answer(
        &self,
        model: &str,
        prompt: &str,
        max_tokens: u32,
    ) -> Result<String, RunnerError> {
        let reply = self
            .client
            .post(format!("{}/api/generate", self.base))
            .timeout(ANSWER_TIMEOUT)
            .json(&json!({
                "model": model,
                "prompt": prompt,
                "stream": false,
                "options": { "num_predict": max_tokens, "num_ctx": CONTEXT_TOKENS },
            }))
            .send()
            .await
            .map_err(|e| RunnerError::Failed(format!("Ollama did not answer: {e}")))?;
        if !reply.status().is_success() {
            let status = reply.status();
            let detail = reply
                .json::<OllamaError>()
                .await
                .map(|e| e.error)
                .unwrap_or_else(|_| status.to_string());
            let hint = if detail.contains("not found") {
                format!("; run `ollama pull {model}` on this machine")
            } else {
                String::new()
            };
            return Err(RunnerError::Failed(format!("Ollama: {detail}{hint}")));
        }
        let generated: Generated = reply
            .json()
            .await
            .map_err(|e| RunnerError::Failed(format!("Ollama sent an unexpected reply: {e}")))?;
        Ok(generated
            .response
            .trim()
            .chars()
            .take(MAX_ANSWER_CHARS)
            .collect())
    }
}

#[async_trait]
impl Runner for LlmRunner {
    fn kind(&self) -> RunnerKind {
        RunnerKind::Llm
    }

    fn models(&self) -> Vec<String> {
        self.models.clone()
    }

    async fn run(
        &self,
        payload: &TaskPayload,
        progress: &mpsc::Sender<u32>,
    ) -> Result<(), RunnerError> {
        // Nobody reads the answers this way; the agent uses run_with_outputs.
        let (outputs, _) = mpsc::channel(1);
        self.run_with_outputs(payload, progress, &outputs).await
    }

    async fn run_with_outputs(
        &self,
        payload: &TaskPayload,
        progress: &mpsc::Sender<u32>,
        outputs: &mpsc::Sender<(u32, String)>,
    ) -> Result<(), RunnerError> {
        let TaskPayload::Llm {
            model,
            frame_start,
            frame_end,
            max_tokens,
            ..
        } = payload
        else {
            return Err(RunnerError::InvalidTask("not an LLM task".into()));
        };
        if !self.models.contains(model) {
            return Err(RunnerError::Failed(format!(
                "model {model} is not installed here; run `ollama pull {model}`"
            )));
        }
        for (done, frame) in (*frame_start..=*frame_end).enumerate() {
            let prompt = payload
                .prompt(frame)
                .ok_or_else(|| RunnerError::InvalidTask(format!("no prompt {frame}")))?;
            let text = self
                .answer(model, prompt, max_tokens.unwrap_or(DEFAULT_ANSWER_TOKENS))
                .await?;
            // A closed receiver just means nobody is listening any more.
            let _ = outputs.send((frame, text)).await;
            let _ = progress.send(done as u32 + 1).await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::{
        Json, Router,
        http::StatusCode,
        routing::{get, post},
    };
    use serde_json::Value;

    use super::*;

    /// A stand-in for Ollama: lists `models`, answers "<model>: <prompt>"
    /// and records what it was asked.
    async fn fake_ollama(models: &[&str]) -> (String, Arc<Mutex<Vec<Value>>>) {
        let tags =
            json!({ "models": models.iter().map(|m| json!({ "name": m })).collect::<Vec<_>>() });
        let installed: Vec<String> = models.iter().map(|m| (*m).to_owned()).collect();
        let asked = Arc::new(Mutex::new(Vec::new()));
        let log = asked.clone();
        let app = Router::new()
            .route("/api/tags", get(move || async move { Json(tags) }))
            .route(
                "/api/generate",
                post(move |Json(body): Json<Value>| async move {
                    log.lock().unwrap().push(body.clone());
                    let model = body["model"].as_str().unwrap_or("").to_owned();
                    if !installed.contains(&model) {
                        let error = json!({ "error": format!("model '{model}' not found") });
                        return (StatusCode::NOT_FOUND, Json(error));
                    }
                    let prompt = body["prompt"].as_str().unwrap_or("");
                    (
                        StatusCode::OK,
                        Json(json!({ "response": format!(" {model}: {prompt} ") })),
                    )
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (base, asked)
    }

    fn job(model: &str, start: u32, end: u32) -> TaskPayload {
        TaskPayload::Llm {
            model: model.into(),
            prompts: vec!["one".into(), "two".into(), "three".into()],
            frame_start: start,
            frame_end: end,
            first_prompt: 1,
            max_tokens: None,
        }
    }

    #[tokio::test]
    async fn answers_its_share_of_prompts_in_order() {
        let (base, asked) = fake_ollama(&["tiny:1b", "bad name"]).await;
        let runner = LlmRunner::detect(&base).await.unwrap();
        assert_eq!(
            runner.models(),
            vec!["tiny:1b".to_owned()],
            "odd names left out"
        );

        let (progress, mut done) = mpsc::channel(8);
        let (outputs, mut answers) = mpsc::channel(8);
        runner
            .run_with_outputs(&job("tiny:1b", 2, 3), &progress, &outputs)
            .await
            .unwrap();
        drop((progress, outputs));
        let mut got = Vec::new();
        while let Some(a) = answers.recv().await {
            got.push(a);
        }
        assert_eq!(
            got,
            vec![
                (2, "tiny:1b: two".to_owned()),
                (3, "tiny:1b: three".to_owned())
            ]
        );
        let mut counts = Vec::new();
        while let Some(n) = done.recv().await {
            counts.push(n);
        }
        assert_eq!(counts, vec![1, 2]);
        let body = &asked.lock().unwrap()[0];
        assert_eq!(body["stream"], false);
        assert_eq!(body["options"]["num_predict"], DEFAULT_ANSWER_TOKENS);
        assert_eq!(body["options"]["num_ctx"], CONTEXT_TOKENS);
    }

    #[tokio::test]
    async fn a_piece_answers_its_own_prompts_at_the_length_asked() {
        let (base, asked) = fake_ollama(&["tiny:1b"]).await;
        let runner = LlmRunner::detect(&base).await.unwrap();
        let whole = TaskPayload::Llm {
            model: "tiny:1b".into(),
            prompts: vec!["one".into(), "two".into(), "three".into()],
            frame_start: 1,
            frame_end: 3,
            first_prompt: 1,
            max_tokens: Some(600),
        };
        let (progress, _p) = mpsc::channel(8);
        let (outputs, mut answers) = mpsc::channel(8);
        runner
            .run_with_outputs(&whole.with_frames(3, 3), &progress, &outputs)
            .await
            .unwrap();
        drop(outputs);
        assert_eq!(answers.recv().await, Some((3, "tiny:1b: three".to_owned())));
        assert_eq!(asked.lock().unwrap()[0]["options"]["num_predict"], 600);
    }

    #[tokio::test]
    async fn a_missing_model_says_how_to_get_it() {
        let (base, _) = fake_ollama(&["tiny:1b"]).await;
        let runner = LlmRunner::detect(&base).await.unwrap();
        let (progress, _p) = mpsc::channel(8);
        let error = runner
            .run(&job("other:7b", 1, 1), &progress)
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("ollama pull other:7b"),
            "{error}"
        );
    }

    #[tokio::test]
    async fn no_server_or_no_models_means_no_runner() {
        // Nothing listens on a port that was just freed.
        let free = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", free.local_addr().unwrap());
        drop(free);
        assert!(LlmRunner::detect(&base).await.is_none());

        let (empty, _) = fake_ollama(&[]).await;
        assert!(LlmRunner::detect(&empty).await.is_none());
    }

    #[tokio::test]
    async fn other_payloads_are_refused() {
        let (base, _) = fake_ollama(&["tiny:1b"]).await;
        let runner = LlmRunner::detect(&base).await.unwrap();
        let (progress, _p) = mpsc::channel(1);
        let fake = TaskPayload::Fake {
            frame_start: 1,
            frame_end: 1,
            millis_per_frame: 0,
        };
        assert!(matches!(
            runner.run(&fake, &progress).await,
            Err(RunnerError::InvalidTask(_))
        ));
    }
}
