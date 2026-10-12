#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use serde::Serialize;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{fs, path::{Component, Path, PathBuf}, process::Command};

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
                    let rel = p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/");
                    saida.push(Arq { path: format!("{}/{}", raiz, rel), text: texto });
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
        || relative.components().any(|part| matches!(part, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
    {
        return Err("Caminho inválido para exclusão".into());
    }
    let candidate = base.join(relative);
    let metadata = fs::symlink_metadata(&candidate).map_err(|e| e.to_string())?;
    let target = candidate.canonicalize().map_err(|e| e.to_string())?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || !target.starts_with(&base)
        || target.extension().and_then(|ext| ext.to_str()).map(|ext| ext.eq_ignore_ascii_case("md")) != Some(true)
    {
        return Err("Só é permitido excluir arquivos Markdown dentro do Vault".into());
    }
    fs::remove_file(candidate).map_err(|e| e.to_string())
}


fn git(root: &str, args: &[&str]) -> Result<String, String> {
    let o = Command::new("git")
        .args(args)
        .current_dir(root)
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

#[tauri::command]
fn git_status(root: String) -> Result<String, String> {
    git(&root, &["status", "--porcelain", "-b"])
}

#[tauri::command]
fn git_log(root: String) -> Result<String, String> {
    git(&root, &["log", "--pretty=format:%h|%ad|%s", "--date=short", "-n", "15"])
}

#[tauri::command]
fn git_init(root: String) -> Result<String, String> {
    git(&root, &["init"])
}

#[tauri::command]
fn git_commit(root: String, msg: String) -> Result<String, String> {
    // os .bak de segurança não entram no repositório
    let gi = Path::new(&root).join(".gitignore");
    let atual = fs::read_to_string(&gi).unwrap_or_default();
    if !atual.contains("*.bak") {
        let sep = if atual.is_empty() || atual.ends_with('\n') { "" } else { "\n" };
        let _ = fs::write(&gi, format!("{}{}*.bak\n", atual, sep));
    }
    git(&root, &["add", "-A", "--", "."])?;
    git(&root, &["commit", "-m", &msg])
}

#[tauri::command]
fn git_push(root: String) -> Result<String, String> {
    git(&root, &["push"])
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
    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
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
        .invoke_handler(tauri::generate_handler![list_md, write_md, delete_md, git_status, git_log, git_init, git_commit, git_push, save_asset, read_asset])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Terricola");
}
