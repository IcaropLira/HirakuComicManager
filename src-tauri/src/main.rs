#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use std::{
    fs,
    path::{Component, Path, PathBuf},
    io::Write,
    process::{Command, Stdio},
    time::Duration,
};

#[derive(Serialize)]
struct Arq {
    path: String,
    text: String,
}

fn varrer(base: &Path, dir: &Path, raiz: &str, saida: &mut Vec<Arq>) {
    if let Ok(entradas) = fs::read_dir(dir) {
        for e in entradas.flatten() {
            let p = e.path();
            let nome = e.file_name().to_string_lossy().to_string();
            if nome.starts_with('.') {
                continue; // ignora .obsidian e afins
            }
            if p.is_dir() {
                varrer(base, &p, raiz, saida);
            } else if nome.to_lowercase().ends_with(".md") {
                if let Ok(texto) = fs::read_to_string(&p) {
                    let rel = p
                        .strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    saida.push(Arq {
                        path: format!("{}/{}", raiz, rel),
                        text: texto,
                    });
                }
            }
        }
    }
}

#[tauri::command]
fn list_md(root: String) -> Vec<Arq> {
    let base = Path::new(&root);
    let nome = base
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "vault".into());
    let mut saida = vec![];
    varrer(base, base, &nome, &mut saida);
    saida
}

#[tauri::command]
fn write_md(path: String, text: String) -> Result<(), String> {
    let p = Path::new(&path);
    if p.exists() {
        let _ = fs::copy(p, format!("{}.bak", path)); // cópia de segurança antes de salvar
    } else if let Some(pasta) = p.parent() {
        fs::create_dir_all(pasta).map_err(|e| e.to_string())?; // notas novas: cria a pasta se preciso
    }
    fs::write(p, text).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_md(root: String, path: String) -> Result<(), String> {
    let base = Path::new(&root).canonicalize().map_err(|e| e.to_string())?;
    let relative = Path::new(&path);
    if relative.is_absolute()
        || relative.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("Caminho inválido para exclusão".into());
    }
    let candidate = base.join(relative);
    let metadata = fs::symlink_metadata(&candidate).map_err(|e| e.to_string())?;
    let target = candidate.canonicalize().map_err(|e| e.to_string())?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || !target.starts_with(&base)
        || target
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("md"))
            != Some(true)
    {
        return Err("Só é permitido excluir arquivos Markdown dentro do Vault".into());
    }
    fs::remove_file(candidate).map_err(|e| e.to_string())
}

fn git(root: &str, args: &[&str]) -> Result<String, String> {
    let o = Command::new("git")
        .args(args)
        .current_dir(root)
        // nunca travar esperando usuário/senha no terminal: falha com erro claro
        .env("GIT_TERMINAL_PROMPT", "0")
        // mensagens do git em inglês: o app compara textos como "No such remote"
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| format!("Não consegui executar o git: {}", e))?;
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    if o.status.success() {
        Ok(if out.is_empty() { err } else { out })
    } else {
        Err(if err.is_empty() { out } else { err })
    }
}

#[tauri::command(async)]
fn git_status(root: String) -> Result<String, String> {
    git(&root, &["status", "--porcelain", "-b"])
}

fn github_repo_path(url: &str) -> Option<&str> {
    url.strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))
}

fn valid_github_repo_url(url: &str) -> bool {
    let Some(path) = github_repo_path(url.trim()) else {
        return false;
    };
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut parts = path.split('/');
    matches!((parts.next(), parts.next(), parts.next()), (Some(owner), Some(repo), None) if !owner.is_empty() && !repo.is_empty())
        && !url.chars().any(char::is_whitespace)
}

#[tauri::command(async)]
fn git_clone(parent: String, url: String) -> Result<String, String> {
    let url = url.trim();
    if !valid_github_repo_url(url) {
        return Err("URL GitHub inválido".into());
    }
    let repo = url
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .strip_suffix(".git")
        .unwrap_or_else(|| url.rsplit('/').next().unwrap_or_default());
    if repo.is_empty()
        || !repo
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    {
        return Err("Nome do repositório inválido".into());
    }
    let parent = Path::new(&parent)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let destination = parent.join(repo);
    if destination.exists() {
        return Err(format!(
            "A pasta {} já existe; escolha outra pasta pai",
            destination.display()
        ));
    }
    let parent = parent.to_str().ok_or("Caminho de destino inválido")?;
    let destination = destination.to_string_lossy().to_string();
    git(parent, &["clone", "--", url, &destination])?;
    Ok(destination)
}

#[tauri::command(async)]
fn git_remote(root: String) -> Result<String, String> {
    match git(&root, &["remote", "get-url", "origin"]) {
        Ok(url) => Ok(url.trim().to_string()),
        Err(err) if err.contains("No such remote") => Ok(String::new()),
        Err(err) => Err(err),
    }
}

#[tauri::command(async)]
fn git_set_remote(root: String, url: String) -> Result<String, String> {
    let url = url.trim();
    if !valid_github_repo_url(url) {
        return Err(
            "Use um URL GitHub no formato https://github.com/usuario/repositorio.git".into(),
        );
    }

    match git(&root, &["remote", "get-url", "origin"]) {
        Ok(_) => git(&root, &["remote", "set-url", "origin", url])?,
        Err(_) => git(&root, &["remote", "add", "origin", url])?,
    };
    Ok(url.to_string())
}

#[tauri::command(async)]
fn git_log(root: String) -> Result<String, String> {
    git(
        &root,
        &[
            "log",
            "--pretty=format:%h|%ad|%s",
            "--date=short",
            "-n",
            "15",
        ],
    )
}

#[tauri::command(async)]
fn git_init(root: String) -> Result<String, String> {
    git(&root, &["init"])
}

#[tauri::command(async)]
fn git_commit(root: String, msg: String) -> Result<String, String> {
    // os .bak de segurança não entram no repositório
    let gi = Path::new(&root).join(".gitignore");
    let atual = fs::read_to_string(&gi).unwrap_or_default();
    if !atual.contains("*.bak") {
        let sep = if atual.is_empty() || atual.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        let _ = fs::write(&gi, format!("{}{}*.bak\n", atual, sep));
    }
    git(&root, &["add", "-A", "--", "."])?;
    // nada para commitar não é erro (senão o push automático seria pulado)
    if git(&root, &["diff", "--cached", "--quiet"]).is_ok() {
        return Ok("Nada para commitar".into());
    }
    git(&root, &["commit", "-m", &msg])
}

#[tauri::command(async)]
fn git_push(root: String) -> Result<String, String> {
    git(&root, &["push", "--set-upstream", "origin", "HEAD"])
}

#[tauri::command(async)]
fn git_pull(root: String) -> Result<String, String> {
    git(&root, &["fetch", "origin"])?;
    let branch = git(&root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let branch = branch.trim();
    if branch == "HEAD" || branch.is_empty() {
        return Err("Faça um commit local antes de sincronizar este repositório".into());
    }
    // remoto ainda sem este branch (primeiro push): não há o que puxar
    if git(
        &root,
        &["rev-parse", "--verify", "--quiet", &format!("refs/remotes/origin/{}", branch)],
    )
    .is_err()
    {
        return Ok("O remoto ainda não tem este branch; nada para puxar".into());
    }
    match git(
        &root,
        &["pull", "--rebase", "--autostash", "origin", branch],
    ) {
        Ok(out) => Ok(out),
        Err(err) => {
            // conflito: desfaz o rebase para o Vault não ficar com marcadores <<<<<<<
            let conflitos = git(&root, &["diff", "--name-only", "--diff-filter=U"])
                .unwrap_or_default()
                .trim()
                .replace('\n', ", ");
            let _ = git(&root, &["rebase", "--abort"]);
            if conflitos.is_empty() {
                Err(err)
            } else {
                Err(format!(
                    "Conflito de merge em: {}. Nada foi alterado no seu Vault; resolva fora do app (git pull --rebase) ou combine com quem editou o mesmo arquivo.",
                    conflitos
                ))
            }
        }
    }
}

#[derive(Serialize)]
struct RemoteState {
    remote: bool,
    branch: String,
    ahead: u32,
    behind: u32,
    dirty: bool,
}

/// Busca o remoto e diz quantos commits o Vault local está atrás/à frente.
#[tauri::command(async)]
fn git_check_remote(root: String) -> Result<RemoteState, String> {
    let vazio = RemoteState {
        remote: false,
        branch: String::new(),
        ahead: 0,
        behind: 0,
        dirty: false,
    };
    if git(&root, &["remote", "get-url", "origin"]).is_err() {
        return Ok(vazio);
    }
    git(&root, &["fetch", "origin"])?;
    let branch = git(&root, &["rev-parse", "--abbrev-ref", "HEAD"])?
        .trim()
        .to_string();
    if branch == "HEAD" || branch.is_empty() {
        return Ok(vazio);
    }
    let dirty = !git(&root, &["status", "--porcelain"])?.trim().is_empty();
    let range = format!("{}...origin/{}", branch, branch);
    let (ahead, behind) = match git(&root, &["rev-list", "--left-right", "--count", &range]) {
        Ok(s) => {
            let mut it = s.split_whitespace().map(|n| n.parse::<u32>().unwrap_or(0));
            (it.next().unwrap_or(0), it.next().unwrap_or(0))
        }
        // branch ainda não existe no remoto
        Err(_) => (0, 0),
    };
    Ok(RemoteState {
        remote: true,
        branch,
        ahead,
        behind,
        dirty,
    })
}

#[tauri::command(async)]
fn git_sync(root: String) -> Result<String, String> {
    let pulled = git_pull(root.clone())?;
    let pushed = git_push(root)?;
    Ok(format!("{}\n{}", pulled, pushed))
}

// ---------- Login no GitHub (Device Flow) ----------
// 1) Crie um OAuth App em github.com/settings/developers e marque "Enable Device Flow".
// 2) Cole o Client ID abaixo (ele é público, não é segredo) ou defina HIRAKU_GITHUB_CLIENT_ID no build.
const GITHUB_CLIENT_ID: &str = match option_env!("HIRAKU_GITHUB_CLIENT_ID") {
    Some(id) => id,
    None => "Ov23lijr9wfVGkHJeXeb",
};
const GITHUB_WEB: &str = "https://github.com";
const GITHUB_API: &str = "https://api.github.com";
const GITHUB_SCOPE: &str = "repo";

#[derive(Serialize)]
struct DeviceCode {
    device_code: String,
    user_code: String,
    verification_uri: String,
    interval: u64,
    expires_in: u64,
}

#[derive(Serialize)]
struct LoginPoll {
    /// "pending" | "slow_down" | "ok" | "expired" | "denied"
    status: String,
    login: Option<String>,
    interval: Option<u64>,
}

fn http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .user_agent("HirakuComicManager")
        .build()
}

fn device_start(web: &str, client_id: &str) -> Result<DeviceCode, String> {
    if client_id.starts_with("COLE_AQUI") || client_id.trim().is_empty() {
        return Err("CLIENT_ID_AUSENTE".into());
    }
    let v: serde_json::Value = http_agent()
        .post(&format!("{}/login/device/code", web))
        .set("Accept", "application/json")
        .send_form(&[("client_id", client_id), ("scope", GITHUB_SCOPE)])
        .map_err(|e| format!("Não consegui falar com o GitHub: {}", e))?
        .into_json()
        .map_err(|e| e.to_string())?;
    if let Some(err) = v["error"].as_str() {
        return Err(format!(
            "O GitHub recusou o login ({}). Confira se o OAuth App tem \"Enable Device Flow\" marcado.",
            err
        ));
    }
    Ok(DeviceCode {
        device_code: v["device_code"].as_str().unwrap_or_default().into(),
        user_code: v["user_code"].as_str().unwrap_or_default().into(),
        verification_uri: v["verification_uri"]
            .as_str()
            .unwrap_or("https://github.com/login/device")
            .into(),
        interval: v["interval"].as_u64().unwrap_or(5),
        expires_in: v["expires_in"].as_u64().unwrap_or(900),
    })
}

/// Ok(Some(token)) quando autorizado; Ok(None) + status quando ainda não.
fn device_poll(web: &str, client_id: &str, device_code: &str) -> Result<Result<String, LoginPoll>, String> {
    let v: serde_json::Value = http_agent()
        .post(&format!("{}/login/oauth/access_token", web))
        .set("Accept", "application/json")
        .send_form(&[
            ("client_id", client_id),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .map_err(|e| format!("Não consegui falar com o GitHub: {}", e))?
        .into_json()
        .map_err(|e| e.to_string())?;
    if let Some(tok) = v["access_token"].as_str() {
        return Ok(Ok(tok.to_string()));
    }
    let status = match v["error"].as_str().unwrap_or("") {
        "authorization_pending" => "pending",
        "slow_down" => "slow_down",
        "expired_token" => "expired",
        "access_denied" => "denied",
        other => return Err(format!("Resposta inesperada do GitHub: {}", other)),
    };
    Ok(Err(LoginPoll {
        status: status.into(),
        login: None,
        interval: v["interval"].as_u64(),
    }))
}

fn github_user(api: &str, token: &str) -> Result<Option<String>, String> {
    match http_agent()
        .get(&format!("{}/user", api))
        .set("Accept", "application/vnd.github+json")
        .set("Authorization", &format!("Bearer {}", token))
        .call()
    {
        Ok(r) => {
            let v: serde_json::Value = r.into_json().map_err(|e| e.to_string())?;
            Ok(v["login"].as_str().map(|s| s.to_string()))
        }
        Err(ureq::Error::Status(401, _)) | Err(ureq::Error::Status(403, _)) => Ok(None),
        Err(e) => Err(format!("Sem conexão com o GitHub: {}", e)),
    }
}

/// Fala com o credential helper do Git (o mesmo que `git push` usa).
fn git_credential(op: &str, input: &str) -> Result<String, String> {
    let mut child = Command::new("git")
        .args(["credential", op])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Não consegui executar o git: {}", e))?;
    child
        .stdin
        .take()
        .ok_or("sem stdin")?
        .write_all(input.as_bytes())
        .map_err(|e| e.to_string())?;
    let o = child.wait_with_output().map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).to_string())
    }
}

fn stored_token() -> Option<String> {
    let out = git_credential("fill", "protocol=https\nhost=github.com\n\n").ok()?;
    out.lines()
        .find_map(|l| l.strip_prefix("password=").map(|s| s.to_string()))
        .filter(|t| !t.is_empty())
}

/// Garante que exista algum credential helper para o Git lembrar do login.
fn ensure_credential_helper() {
    let tem = Command::new("git")
        .args(["config", "--get-all", "credential.helper"])
        .output()
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false);
    if tem {
        return;
    }
    let helper = if cfg!(target_os = "windows") {
        "manager"
    } else if cfg!(target_os = "macos") {
        "osxkeychain"
    } else if ["/usr/libexec/git-core/git-credential-libsecret", "/usr/lib/git-core/git-credential-libsecret"]
        .iter()
        .any(|p| Path::new(p).exists())
    {
        "libsecret"
    } else {
        "store"
    };
    let _ = Command::new("git")
        .args(["config", "--global", "credential.helper", helper])
        .output();
}

#[tauri::command(async)]
fn github_login_start() -> Result<DeviceCode, String> {
    device_start(GITHUB_WEB, GITHUB_CLIENT_ID)
}

#[tauri::command(async)]
fn github_login_poll(device_code: String) -> Result<LoginPoll, String> {
    match device_poll(GITHUB_WEB, GITHUB_CLIENT_ID, &device_code)? {
        Err(pending) => Ok(pending),
        Ok(token) => {
            let login = github_user(GITHUB_API, &token)?.ok_or("O GitHub não aceitou o token recebido")?;
            ensure_credential_helper();
            git_credential(
                "approve",
                &format!(
                    "protocol=https\nhost=github.com\nusername={}\npassword={}\n\n",
                    login, token
                ),
            )?;
            Ok(LoginPoll {
                status: "ok".into(),
                login: Some(login),
                interval: None,
            })
        }
    }
}

/// Login da conta conectada (None se não houver login salvo ou se o token expirou).
#[tauri::command(async)]
fn github_account() -> Result<Option<String>, String> {
    let Some(token) = stored_token() else {
        return Ok(None);
    };
    github_user(GITHUB_API, &token)
}

#[tauri::command(async)]
fn github_logout() -> Result<(), String> {
    git_credential("reject", "protocol=https\nhost=github.com\n\n").map(|_| ())
}

#[derive(Serialize)]
struct GithubStarStatus {
    username: String,
    starred: bool,
}

#[tauri::command(async)]
fn github_star_status() -> Result<GithubStarStatus, String> {
    let token = stored_token().ok_or("Entre com o GitHub na aba Git primeiro")?;
    let username = github_user(GITHUB_API, &token)?.ok_or("Login do GitHub expirou; entre de novo")?;
    let r = http_agent()
        .get(&format!("{}/user/starred/IcaropLira/HirakuComicManager", GITHUB_API))
        .set("Authorization", &format!("Bearer {}", token))
        .call();
    match r {
        Ok(_) => Ok(GithubStarStatus { username, starred: true }),
        Err(ureq::Error::Status(404, _)) => Ok(GithubStarStatus { username, starred: false }),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn open_github_repository() -> Result<(), String> {
    open_url("https://github.com/IcaropLira/HirakuComicManager".into())
}

/// Abre no navegador padrão. Só aceita endereços do github.com.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://github.com/") || url.chars().any(|c| c.is_whitespace() || c == '"' || c == '&' || c == '^') {
        return Err("Endereço não permitido".into());
    }
    let url = url.as_str();
    #[cfg(target_os = "linux")]
    let result = Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let result = Command::new("cmd").args(["/C", "start", "", url]).spawn();
    result.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_asset(path: String, b64: String) -> Result<(), String> {
    let bytes = STANDARD.decode(b64).map_err(|e| e.to_string())?;
    let p = Path::new(&path);
    if let Some(pasta) = p.parent() {
        fs::create_dir_all(pasta).map_err(|e| e.to_string())?;
    }
    fs::write(p, bytes).map_err(|e| e.to_string())
}

fn achar(dir: &Path, alvo: &str, melhor: &mut Option<PathBuf>) {
    if let Ok(entradas) = fs::read_dir(dir) {
        for e in entradas.flatten() {
            let p = e.path();
            let nome = e.file_name().to_string_lossy().to_string();
            if nome.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                achar(&p, alvo, melhor);
            } else {
                let cheio = p.to_string_lossy().replace('\\', "/").to_lowercase();
                let base = alvo.rsplit('/').next().unwrap_or(alvo);
                if cheio.ends_with(&format!("/{}", alvo)) {
                    *melhor = Some(p.clone()); // casa o caminho completo do link
                } else if melhor.is_none() && nome.to_lowercase() == base {
                    *melhor = Some(p.clone()); // casa só o nome do arquivo
                }
            }
        }
    }
}

#[tauri::command]
fn read_asset(root: String, link: String) -> Result<String, String> {
    let alvo = link.replace('\\', "/").to_lowercase();
    let mut m = None;
    achar(Path::new(&root), &alvo, &mut m);
    let p = m.ok_or("arquivo não encontrado")?;
    let bytes = fs::read(&p).map_err(|e| e.to_string())?;
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    };
    Ok(format!("data:{};base64,{}", mime, STANDARD.encode(bytes)))
}

fn main() {
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            list_md,
            write_md,
            delete_md,
            git_status,
            git_clone,
            git_remote,
            git_set_remote,
            git_log,
            git_init,
            git_commit,
            git_push,
            git_pull,
            git_check_remote,
            git_sync,
            github_login_start,
            github_login_poll,
            github_account,
            github_logout,
            github_star_status,
            open_github_repository,
            open_url,
            save_asset,
            read_asset
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Terricola");
}
