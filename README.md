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

O botão de sincronização faz `fetch` e `pull --rebase --autostash`, depois envia o branch atual. Se houver conflito de merge, o app desfaz o rebase (seu Vault não fica com marcadores `<<<<<<<`) e avisa quais arquivos conflitaram.

Com **Automático** ligado na aba Git:
- ao salvar uma nota o app puxa o remoto, faz commit e push;
- a cada minuto (e ao voltar o foco para a janela) o app consulta o remoto. Se alguém enviou commits, aparece um aviso e, se você não estiver com um editor aberto, o pull/merge é feito sozinho e o Vault recarrega. Com um editor aberto, o aviso fica até você salvar e sair.
Com o modo desligado, o aviso aparece com o botão **Puxar e mesclar**.

## Releases

Envie uma tag `v*` para publicar artefatos para Linux (DEB, RPM e AppImage), Windows (MSI e NSIS) e macOS (DMG). O workflow cria o GitHub Release automaticamente.

### Atualização automática do app

O app verifica novas versões ao abrir (e a cada 6 h) e mostra um aviso com **Atualizar agora**; também há "Verificar atualizações" no rodapé. Configuração única:

1. `npx tauri signer generate -w ~/.tauri/hiraku.key` (guarde a chave privada e a senha).
2. Cole a **chave pública** em `src-tauri/tauri.conf.json` → `plugins.updater.pubkey` (hoje há um texto de exemplo no lugar).
3. No GitHub: *Settings → Secrets and variables → Actions* e crie `TAURI_SIGNING_PRIVATE_KEY` (conteúdo da chave privada) e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
4. Publique com uma tag `v*`. O workflow assina os instaladores e envia o `latest.json` ao Release, que é o endereço consultado pelos apps instalados.

Sem a chave pública correta o app abre normalmente, mas a verificação de atualização falha. A atualização silenciosa vale para Windows (NSIS/MSI), macOS e AppImage; instalações `.deb`/`.rpm` precisam ser atualizadas pelo gerenciador do sistema. Flatpak requer manifesto e publicação próprios.

### Instalação

- Windows: baixe o instalador `*-setup.exe` (NSIS) no GitHub Release; ele instala o app e cria os atalhos com o ícone do produto.
- Debian/Ubuntu: instale o arquivo `.deb` para registrar o app no menu do sistema.
- Fedora e derivados: instale o `.rpm` para registrar o app no menu.
- AppImage: opção portátil; permita execução e abra o arquivo diretamente.

O workflow aplica a versão da tag a `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` e `package.json` antes de compilar. Exemplo de publicação: `git tag v0.1.1 && git push origin v0.1.1`.

