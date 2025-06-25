# pcode: Production AI Code Agent - Final Specification v2.0

## Executive Summary

`pcode` is a deterministic, security-first AI code agent achieving <200ms first-token latency through a simplified architecture that prioritizes maintainability without sacrificing performance. Key changes from v1.0:

- **Unified async runtime** using Tokio with custom schedulers, eliminating the triple-runtime complexity
- **Self-contained token estimation** using a 256KB perfect hash table, removing ML dependencies
- **Cross-platform security** via abstraction layer supporting Linux (Landlock), macOS (Sandbox), and Windows (AppContainer)
- **Graceful degradation** for all advanced features with automatic fallback paths

Target metrics: 12MB stripped binary (musl), 20MB RSS baseline, 150ms p99 first-token latency.

## Core Architecture

### Unified Runtime Design

We consolidate to a single Tokio runtime with custom task schedulers for different workload types:

```rust
// src/runtime/unified.rs
pub struct UnifiedRuntime {
    rt: tokio::runtime::Runtime,
    schedulers: SchedulerSet,
}

struct SchedulerSet {
    // UI tasks: low latency, high priority
    ui_scheduler: PriorityScheduler,
    // Tool tasks: CPU-bound, isolated
    tool_scheduler: IsolatedScheduler,
    // LLM tasks: I/O-bound, streaming
    llm_scheduler: StreamScheduler,
}

impl UnifiedRuntime {
    pub fn new() -> Result<Self> {
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        
        // Detect CPU topology but handle heterogeneous architectures
        let topology = CpuTopology::detect();
        
        match topology {
            CpuTopology::Homogeneous(cores) => {
                builder.worker_threads(cores);
                builder.on_thread_start(move || {
                    // Simple round-robin affinity
                    let worker_id = WORKER_COUNTER.fetch_add(1, Ordering::Relaxed);
                    set_cpu_affinity(worker_id % cores);
                });
            }
            CpuTopology::Heterogeneous { p_cores, e_cores } => {
                // Use P-cores for latency-sensitive work
                builder.worker_threads(p_cores.len());
                builder.on_thread_start(move || {
                    let worker_id = WORKER_COUNTER.fetch_add(1, Ordering::Relaxed);
                    // Pin to P-cores only
                    set_cpu_affinity(p_cores[worker_id % p_cores.len()]);
                });
            }
            CpuTopology::Unknown => {
                // No affinity, let OS scheduler decide
                builder.worker_threads(num_cpus::get());
            }
        }
        
        let rt = builder.build()?;
        
        Ok(Self {
            rt,
            schedulers: SchedulerSet::new(),
        })
    }
}

// CPU topology detection with heterogeneous support
#[derive(Debug)]
enum CpuTopology {
    Homogeneous(usize),
    Heterogeneous { p_cores: Vec<usize>, e_cores: Vec<usize> },
    Unknown,
}

impl CpuTopology {
    #[cfg(target_os = "macos")]
    fn detect() -> Self {
        // Use sysctlbyname to detect P/E cores on Apple Silicon
        use std::mem;
        
        let mut p_cores = 0u32;
        let mut p_size = mem::size_of::<u32>();
        
        unsafe {
            if sysctlbyname(
                b"hw.perflevel0.physicalcpu\0".as_ptr() as *const _,
                &mut p_cores as *mut _ as *mut _,
                &mut p_size,
                std::ptr::null_mut(),
                0
            ) == 0 {
                let mut e_cores = 0u32;
                let mut e_size = mem::size_of::<u32>();
                
                if sysctlbyname(
                    b"hw.perflevel1.physicalcpu\0".as_ptr() as *const _,
                    &mut e_cores as *mut _ as *mut _,
                    &mut e_size,
                    std::ptr::null_mut(),
                    0
                ) == 0 && e_cores > 0 {
                    // Apple Silicon with P+E cores
                    return CpuTopology::Heterogeneous {
                        p_cores: (0..p_cores as usize).collect(),
                        e_cores: (p_cores as usize..(p_cores + e_cores) as usize).collect(),
                    };
                }
            }
        }
        
        CpuTopology::Homogeneous(num_cpus::get())
    }
    
    #[cfg(target_os = "linux")]
    fn detect() -> Self {
        // Check for Intel hybrid via cpuinfo
        if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
            // Detect Intel 12th gen+ hybrid
            if cpuinfo.contains("core_type") {
                // Parse P/E cores from sysfs
                // Implementation omitted for brevity
            }
        }
        
        CpuTopology::Homogeneous(num_cpus::get())
    }
    
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    fn detect() -> Self {
        CpuTopology::Unknown
    }
}
```

### Efficient Work Distribution

Replace spin-waiting with async coordination:

```rust
// src/runtime/scheduler.rs
pub struct IsolatedScheduler {
    queue: Arc<SegQueue<ToolTask>>,
    notify: Arc<Notify>,
}

impl IsolatedScheduler {
    pub async fn submit(&self, task: ToolTask) -> ToolResult {
        let (tx, rx) = oneshot::channel();
        self.queue.push(ToolTask { inner: task, result: tx });
        self.notify.notify_one();
        rx.await.unwrap()
    }
    
    pub async fn worker_loop(self: Arc<Self>) {
        loop {
            // Async wait instead of spin
            if self.queue.is_empty() {
                self.notify.notified().await;
            }
            
            if let Some(task) = self.queue.pop() {
                // Use spawn_blocking for CPU-bound work
                let result = tokio::task::spawn_blocking(move || {
                    execute_tool_isolated(task.inner)
                }).await.unwrap();
                
                let _ = task.result.send(result);
            }
        }
    }
}
```

## Self-Contained Token Estimation

Replace the neural network with a compact, deterministic approach:

```rust
// src/context/token_counter.rs
pub struct CompactTokenCounter {
    // 256KB lookup table for common patterns
    pattern_table: Box<[u16; 131072]>, // 2^17 entries
    // Simple BPE rules for fallback
    bpe_rules: BpeRuleset,
}

impl CompactTokenCounter {
    pub fn new() -> Self {
        // Build at compile time from a minimal BPE vocabulary
        const TABLE: [u16; 131072] = include!(concat!(env!("OUT_DIR"), "/token_table.rs"));
        
        Self {
            pattern_table: Box::new(TABLE),
            bpe_rules: BpeRuleset::minimal(),
        }
    }
    
    pub fn count_tokens(&self, text: &str) -> usize {
        let mut tokens = 0;
        let bytes = text.as_bytes();
        let mut i = 0;
        
        while i < bytes.len() {
            // Try to match longest pattern in lookup table
            let mut matched = false;
            
            for len in (1..=8).rev() {
                if i + len <= bytes.len() {
                    let hash = xxhash_rust::xxh3::xxh3_64(&bytes[i..i+len]) as usize;
                    let index = hash & 0x1FFFF; // Mask to 17 bits
                    
                    if self.pattern_table[index] != 0 {
                        tokens += self.pattern_table[index] as usize;
                        i += len;
                        matched = true;
                        break;
                    }
                }
            }
            
            if !matched {
                // Fallback to simple BPE
                let (token_count, bytes_consumed) = self.bpe_rules.tokenize_at(&bytes[i..]);
                tokens += token_count;
                i += bytes_consumed;
            }
        }
        
        tokens
    }
}

// Minimal BPE implementation (2KB of rules)
struct BpeRuleset {
    merges: Vec<(Vec<u8>, Vec<u8>, u16)>, // (left, right, token_count)
}

impl BpeRuleset {
    fn minimal() -> Self {
        // Include only the most common 1000 merges
        Self {
            merges: vec![
                (b"th".to_vec(), b"e".to_vec(), 1),
                (b"in".to_vec(), b"g".to_vec(), 1),
                // ... generated from corpus analysis
            ],
        }
    }
}
```

Build script generates the lookup table:

```rust
// build.rs
fn generate_token_table() {
    let mut table = [0u16; 131072];
    
    // Analyze common code patterns
    let patterns = analyze_code_corpus("data/code_corpus.txt");
    
    for (pattern, token_count) in patterns {
        let hash = xxhash_rust::xxh3::xxh3_64(pattern.as_bytes()) as usize;
        let index = hash & 0x1FFFF;
        
        // Handle collisions by keeping lower token count
        if table[index] == 0 || token_count < table[index] {
            table[index] = token_count.min(u16::MAX);
        }
    }
    
    // Write table
    let out_path = Path::new(&env::var("OUT_DIR").unwrap()).join("token_table.rs");
    let mut file = File::create(out_path).unwrap();
    write!(file, "[").unwrap();
    for (i, &count) in table.iter().enumerate() {
        if i > 0 { write!(file, ",").unwrap(); }
        write!(file, "{}", count).unwrap();
    }
    write!(file, "]").unwrap();
}
```

## Tool Discovery with Fallback

Implement a robust discovery mechanism with multiple strategies:

```rust
// src/mcp/discovery.rs
pub struct RobustToolDiscovery {
    strategies: Vec<Box<dyn DiscoveryStrategy>>,
}

#[async_trait]
trait DiscoveryStrategy: Send + Sync {
    async fn discover(&self) -> Result<Vec<ToolManifest>>;
    fn priority(&self) -> u8;
}

impl RobustToolDiscovery {
    pub fn new() -> Self {
        let mut strategies: Vec<Box<dyn DiscoveryStrategy>> = vec![];
        
        // Try eBPF first (fastest, most secure)
        #[cfg(target_os = "linux")]
        if let Ok(ebpf) = EbpfDiscovery::new() {
            strategies.push(Box::new(ebpf));
        }
        
        // Filesystem watcher as fallback
        strategies.push(Box::new(FsWatcherDiscovery::new()));
        
        // PATH scanning as last resort
        strategies.push(Box::new(PathScanDiscovery::new()));
        
        strategies.sort_by_key(|s| std::cmp::Reverse(s.priority()));
        
        Self { strategies }
    }
    
    pub async fn discover_all(&self) -> Vec<ToolManifest> {
        for strategy in &self.strategies {
            match strategy.discover().await {
                Ok(tools) if !tools.is_empty() => {
                    tracing::info!("Discovered {} tools using {}", tools.len(), strategy.name());
                    return tools;
                }
                Err(e) => {
                    tracing::warn!("Discovery strategy {} failed: {}", strategy.name(), e);
                }
                _ => continue,
            }
        }
        
        Vec::new()
    }
}

// Filesystem watcher using notify
struct FsWatcherDiscovery {
    watcher: Arc<Mutex<RecommendedWatcher>>,
    known_tools: Arc<RwLock<HashMap<PathBuf, ToolManifest>>>,
}

impl FsWatcherDiscovery {
    fn new() -> Self {
        let known_tools = Arc::new(RwLock::new(HashMap::new()));
        let known_tools_clone = known_tools.clone();
        
        let (tx, rx) = channel();
        let watcher = notify::watcher(tx, Duration::from_secs(2)).unwrap();
        
        // Watch common tool directories
        for dir in &["/usr/local/bin", "~/.local/bin", "~/.cargo/bin"] {
            if let Ok(path) = shellexpand::full(dir) {
                let _ = watcher.watch(Path::new(path.as_ref()), RecursiveMode::NonRecursive);
            }
        }
        
        // Background task to process events
        tokio::spawn(async move {
            while let Ok(event) = rx.recv() {
                if let DebouncedEvent::Create(path) | DebouncedEvent::Write(path) = event {
                    if let Ok(manifest) = probe_tool_manifest(&path).await {
                        known_tools_clone.write().await.insert(path, manifest);
                    }
                }
            }
        });
        
        Self {
            watcher: Arc::new(Mutex::new(watcher)),
            known_tools,
        }
    }
}
```

## Cross-Platform Security Model

Abstract security primitives across platforms:

```rust
// src/security/mod.rs
pub trait SecuritySandbox: Send + Sync {
    fn apply_restrictions(&self, policy: &SecurityPolicy) -> Result<()>;
    fn verify_manifest(&self, manifest: &[u8], signature: &[u8]) -> Result<()>;
}

#[cfg(target_os = "linux")]
pub type PlatformSandbox = LinuxSandbox;

#[cfg(target_os = "macos")]
pub type PlatformSandbox = MacOsSandbox;

#[cfg(target_os = "windows")]
pub type PlatformSandbox = WindowsSandbox;

// Linux implementation with Landlock
#[cfg(target_os = "linux")]
struct LinuxSandbox {
    landlock: landlock::Ruleset,
    seccomp: seccomp::Filter,
}

#[cfg(target_os = "linux")]
impl SecuritySandbox for LinuxSandbox {
    fn apply_restrictions(&self, policy: &SecurityPolicy) -> Result<()> {
        // Apply Landlock rules
        let mut ruleset = self.landlock.create()?;
        
        for path in &policy.allowed_reads {
            ruleset = ruleset.add_rule(landlock::PathBeneath::new(
                path,
                landlock::AccessFs::ReadFile | landlock::AccessFs::ReadDir,
            ))?;
        }
        
        ruleset.restrict_self()?;
        
        // Apply seccomp filters
        self.seccomp.load()?;
        
        Ok(())
    }
}

// macOS implementation with App Sandbox
#[cfg(target_os = "macos")]
struct MacOsSandbox {
    entitlements: sandbox::Entitlements,
}

#[cfg(target_os = "macos")]
impl SecuritySandbox for MacOsSandbox {
    fn apply_restrictions(&self, policy: &SecurityPolicy) -> Result<()> {
        use sandbox_mac::*;
        
        let mut profile = Profile::new("pcode-tool");
        
        // File access
        for path in &policy.allowed_reads {
            profile.allow_file_read(path);
        }
        
        for path in &policy.allowed_writes {
            profile.allow_file_write(path);
        }
        
        // Network (with domain restrictions)
        if policy.network_access.is_some() {
            for domain in &policy.network_access.allowed_domains {
                profile.allow_network_outbound(domain);
            }
        }
        
        profile.apply()?;
        Ok(())
    }
}

// Windows implementation with AppContainer
#[cfg(target_os = "windows")]
struct WindowsSandbox {
    container: appcontainer::AppContainer,
}

#[cfg(target_os = "windows")]
impl SecuritySandbox for WindowsSandbox {
    fn apply_restrictions(&self, policy: &SecurityPolicy) -> Result<()> {
        use windows::Win32::Security::*;
        
        let mut container = appcontainer::AppContainer::new("pcode-tool")?;
        
        // Set capabilities
        if policy.network_access.is_some() {
            container.add_capability(WinCapabilityNetworkClient);
        }
        
        // File system restrictions via explicit ACLs
        for path in &policy.allowed_reads {
            container.add_filesystem_rule(path, FILE_GENERIC_READ)?;
        }
        
        container.apply_to_current_process()?;
        Ok(())
    }
}
```

## Enhanced Security Policy

Fine-grained network control:

```rust
// src/security/policy.rs
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub allowed_reads: Vec<PathBuf>,
    pub allowed_writes: Vec<PathBuf>,
    pub allowed_creates: Vec<PathBuf>,
    pub network_access: Option<NetworkPolicy>,
    pub resource_limits: ResourceLimits,
    pub capabilities: ToolCapabilities,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkPolicy {
    pub allowed_domains: Vec<String>,
    pub allowed_ports: Vec<u16>,
    pub allowed_protocols: Vec<Protocol>,
    pub dns_servers: Option<Vec<IpAddr>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Protocol {
    Tcp,
    Udp,
    Http,
    Https,
}

// DNS-based egress control
pub struct NetworkEnforcer {
    resolver: trust_dns_resolver::AsyncResolver,
    allowed_ips: Arc<RwLock<HashSet<IpAddr>>>,
}

impl NetworkEnforcer {
    pub async fn resolve_allowed_domains(&self, domains: &[String]) -> Result<()> {
        let mut allowed = HashSet::new();
        
        for domain in domains {
            let response = self.resolver.lookup_ip(domain).await?;
            for ip in response {
                allowed.insert(ip);
            }
        }
        
        *self.allowed_ips.write().await = allowed;
        Ok(())
    }
    
    pub async fn check_connection(&self, addr: &SocketAddr) -> bool {
        self.allowed_ips.read().await.contains(&addr.ip())
    }
}
```

## MCP Protocol Evolution

Support for streaming and manifest verification:

```capnp
# mcp-v2.capnp
@0xdeadbeef12345678;

struct ToolManifest {
    name @0 :Text;
    version @1 :Version;
    capabilities @2 :List(Capability);
    inputSchema @3 :Data;
    outputSchema @4 :Data;
    
    # New: cryptographic identity
    publicKey @5 :Data;  # Ed25519 public key
    signature @6 :Data;  # Self-signature of fields 0-4
    
    # New: streaming support
    streamingSupport @7 :StreamingMode;
}

enum StreamingMode {
    none @0;
    input @1;    # Tool accepts streaming input
    output @2;   # Tool produces streaming output
    both @3;     # Full duplex streaming
}

# Streaming protocol
struct StreamRequest {
    id @0 :UInt64;
    sequence @1 :UInt32;  # For ordering
    data @2 :Data;
    isLast @3 :Bool;
}

struct StreamResponse {
    id @0 :UInt64;
    sequence @1 :UInt32;
    data @2 :Data;
    isLast @3 :Bool;
    error @4 :Text;  # Optional error
}
```

Manifest verification:

```rust
// src/mcp/trust.rs
use ed25519_dalek::{PublicKey, Signature, Verifier};

pub struct TrustManager {
    trusted_keys: HashMap<String, PublicKey>,
}

impl TrustManager {
    pub fn verify_manifest(&self, manifest: &ToolManifest) -> Result<bool> {
        // Extract the public key
        let public_key = PublicKey::from_bytes(&manifest.public_key)?;
        
        // Reconstruct the signed portion
        let mut signed_data = Vec::new();
        signed_data.extend_from_slice(manifest.name.as_bytes());
        signed_data.extend_from_slice(&manifest.version.to_bytes());
        // ... other fields
        
        // Verify signature
        let signature = Signature::from_bytes(&manifest.signature)?;
        Ok(public_key.verify(&signed_data, &signature).is_ok())
    }
}
```

## Secure Build Pipeline

Hardware security module integration:

```rust
// ci/sign.rs
use rusoto_kms::{KmsClient, SignRequest};

async fn sign_binary(binary_path: &Path) -> Result<Vec<u8>> {
    // Read and hash the binary
    let binary_data = tokio::fs::read(binary_path).await?;
    let hash = blake3::hash(&binary_data);
    
    // Use AWS KMS for signing (key never leaves HSM)
    let client = KmsClient::new(Default::default());
    
    let sign_request = SignRequest {
        key_id: env::var("PCODE_KMS_KEY_ID")?,
        message: hash.as_bytes().to_vec().into(),
        message_type: Some("DIGEST".to_string()),
        signing_algorithm: "ECDSA_SHA_256".to_string(),
        ..Default::default()
    };
    
    let response = client.sign(sign_request).await?;
    Ok(response.signature.unwrap().to_vec())
}
```

## Performance Validation Framework

Automated benchmark enforcement:

```rust
// benches/latency.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_first_token_latency(c: &mut Criterion) {
    let runtime = pcode::UnifiedRuntime::new().unwrap();
    
    let mut group = c.benchmark_group("first_token_latency");
    
    for scenario in &["simple_prompt", "with_tool_call", "max_context"] {
        group.bench_with_input(
            BenchmarkId::from_parameter(scenario),
            scenario,
            |b, scenario| {
                b.to_async(&runtime.rt).iter(|| async {
                    let prompt = load_test_prompt(scenario);
                    let start = std::time::Instant::now();
                    
                    let mut stream = runtime.process_prompt(prompt).await;
                    let _first_chunk = stream.next().await.unwrap();
                    
                    start.elapsed()
                });
            },
        );
    }
    
    group.finish();
}

// Enforce performance regression
#[test]
fn test_performance_targets() {
    let results = run_benchmarks();
    
    assert!(results.first_token_p50 < Duration::from_millis(150));
    assert!(results.first_token_p99 < Duration::from_millis(250));
    assert!(results.tool_discovery_p99 < Duration::from_millis(50));
}
```

## Binary Size Optimization

Achieve <12MB target through aggressive optimization:

```toml
# Cargo.toml
[profile.release]
opt-level = "z"          # Optimize for size
lto = true               # Link-time optimization
codegen-units = 1        # Single codegen unit
strip = true             # Strip symbols
panic = "abort"          # No unwinding

[dependencies]
# Use lite versions where possible
tokio = { version = "1.35", default-features = false, features = ["rt-multi-thread", "net", "time"] }
serde = { version = "1.0", default-features = false, features = ["derive"] }
# Avoid heavy dependencies
# NO: ort, ndarray, tensorflow

[build-dependencies]
# Generate lookup tables at compile time
phf_codegen = "0.11"
```

Post-build optimization:

```bash
#!/bin/bash
# scripts/optimize-binary.sh

# Build with musl for static linking
cargo build --release --target x86_64-unknown-linux-musl

# Strip all symbols
strip -s target/x86_64-unknown-linux-musl/release/pcode

# Compress with UPX (lossless)
upx --ultra-brute --best target/x86_64-unknown-linux-musl/release/pcode

# Verify size
SIZE=$(stat -f%z target/x86_64-unknown-linux-musl/release/pcode)
if [ $SIZE -gt 12582912 ]; then  # 12MB in bytes
    echo "ERROR: Binary size ${SIZE} exceeds 12MB limit"
    exit 1
fi
```

## Final Architecture Summary

This specification delivers a production-grade AI code agent that:

1. **Simplifies complexity** through a unified runtime while maintaining performance
2. **Eliminates external dependencies** for token counting with a compact lookup table
3. **Provides robust cross-platform security** with platform-native sandboxing
4. **Ensures reliability** through multiple fallback strategies for every component
5. **Achieves aggressive performance targets** validated through automated benchmarking

The implementation represents approximately 25,000 hours of engineering effort, targeting Q4 2024 release.

## 8. Future Enhancements

### 8.1 Workspace Intelligence
- Project structure understanding
- Dependency graph analysis
- Cross-file refactoring
- Build system integration

### 8.2 Conversation Memory
- Session persistence across runs
- Context retrieval from previous sessions
- Learning from user corrections
- Project-specific knowledge base

### 8.3 Multi-File Operations
- Atomic changes across multiple files
- Transaction support with rollback
- Bulk search and replace
- Coordinated refactoring

### 8.4 Change Management
- Diff preview before applying
- Undo/redo stack
- Change history tracking
- Rollback to previous states

### 8.5 Development Integration
- Debugger integration (breakpoints, stepping)
- Language server protocol (LSP) support
- Real-time error checking
- Intelligent code completion

### 8.6 Version Control
- Git workflow automation
- Branch management
- Commit message generation
- Code review assistance

### 8.7 Extended Language Support
- Ruby, Go, Java, C/C++
- Language-specific security profiles
- Custom tool development SDK

### 8.8 Advanced Features
- Multi-agent collaboration
- Distributed execution
- Cloud deployment options
- Enterprise features (SSO, audit logs)
