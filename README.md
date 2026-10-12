# Hiraku Comics Manager (app desktop)

1. Pré-requisitos do Tauri: https://v2.tauri.app/start/prerequisites/ (Rust + dependências do seu sistema)
2. `npm install`   (também copia o d3 para src/)
3. `npx tauri icon caminho/para/logo.png`   (gera a pasta src-tauri/icons; obrigatório uma vez)
4. `npm run dev`   (abre o app)  ·  `npm run build`  (gera o instalador)

No app, "Escolher pasta…" lê o vault; o app lembra a última pasta.
"Salvar no arquivo" grava a ficha e deixa uma cópia `arquivo.md.bak` ao lado.

## GitHub

Na aba Git, conecte o remoto do Vault. Para autenticação HTTPS, configure o Git Credential Manager ou execute `gh auth login` uma vez no sistema. O app usa as credenciais do Git e não guarda tokens no Vault.

Para começar um projeto, informe o URL do repositório GitHub em **Clonar projeto** na Home e escolha a pasta pai. Se o repositório for privado, faça `gh auth login` antes de clonar. Para um Vault já clonado, configure `origin` na aba Git.

O botão de sincronização faz `fetch` e `pull --rebase --autostash`, depois envia o branch atual. Conflitos interrompem a sincronização para resolução manual. O modo automático é opcional e sincroniza ao salvar notas.

## Releases

Envie uma tag `v*` para publicar artefatos para Linux (DEB, RPM e AppImage), Windows (MSI e NSIS) e macOS (DMG). O workflow cria o GitHub Release automaticamente.

O atualizador silencioso do app ainda precisa do plugin Tauri Updater e de uma chave de assinatura configurada como secret do GitHub Actions. Flatpak também requer manifesto e publicação próprios; o workflow atual não gera `.flatpak`.

### Instalação

- Windows: baixe o instalador `*-setup.exe` (NSIS) no GitHub Release; ele instala o app e cria os atalhos com o ícone do produto.
- Debian/Ubuntu: instale o arquivo `.deb` para registrar o app no menu do sistema.
- Fedora e derivados: instale o `.rpm` para registrar o app no menu.
- AppImage: opção portátil; permita execução e abra o arquivo diretamente.

Os instaladores são gerados quando uma tag corresponde à versão em `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` e `package.json`. Exemplo para a versão atual: `git tag v0.1.0 && git push origin v0.1.0`.

