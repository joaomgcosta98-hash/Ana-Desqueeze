# AnaDesqueeze · João Costa ©

Aplicação para descomprimir fotografias anamórficas no Mac Apple Silicon (M1 ou posterior), macOS 13 ou posterior. Interface inspirada nos mockups **Dark Clean**, **Batch Dark** e **Editor Dark**.

## Nesta versão

- Interface escura em todos os painéis, logótipo próprio no cabeçalho e ícone da aplicação no Dock.
- Vista **Processar** com modos Original, Descomprimida e Comparar, mais uma faixa de fotografias.
- Vista **Lote** com miniaturas, seleção individual, Selecionar todas e Limpar seleção. O botão Exportar mostra quantas fotografias estão selecionadas.
- Importação de ficheiros, pasta (primeiro nível) e arrastar ficheiros para a janela.
- Fatores 1.25×, 1.33×, 1.5×, 1.6×, 1.8×, 2× e personalizado entre 0.5× e 4×.
- Exportação JPEG, PNG e TIFF. Os originais nunca são escritos e cada destino é criado com nome exclusivo.

O visual dos mockups incluía DNG. **A exportação DNG RAW ainda não está implementada**; as saídas atuais são imagens renderizadas. Os formatos RAW e HEIC usam o descodificador `sips` do macOS quando necessário, com suporte dependente da câmara e do sistema. EXIF, perfis ICC e profundidade original não são garantidos. JPEG usa RGB de 8 bits e perde transparência. Confirma a cor antes de usar esta versão num fluxo crítico.

## Atualizar o repositório existente pelo navegador

Repositório: https://github.com/joaomgcosta98-hash/Ana-Desqueeze

1. Descompacta `AnaDesqueeze-visual-batch.zip` e abre a pasta `AnaDesqueeze`.
2. Em **Code → Add file → Upload files**, envia da pasta descompactada `Cargo.toml`, `Cargo.lock`, `README.md` e as pastas `src`, `scripts` e `assets`. Mantém a mesma estrutura na raiz do repositório. Se o GitHub mostrar os ficheiros como alterados, guarda com **Commit changes** na branch `main`.
3. Confirma que `assets/logo.png`, `assets/AnaDesqueeze.icns`, `src/main.rs` e `scripts/package-macos.sh` aparecem nas respetivas pastas.
4. O workflow que já existe em `.github/workflows/mac-installer.yml` arranca quando as alterações são guardadas em `main`. Se não arrancar, abre **Actions → Criar instalador macOS → Run workflow**.
5. Espera por um visto verde. Abre a execução e descarrega **Artifacts → AnaDesqueeze-AppleSilicon**. Extrai o ZIP e abre `AnaDesqueeze-AppleSilicon.dmg`.
6. Arrasta AnaDesqueeze para **Aplicações**, ejeta o disco e abre a aplicação. Como esta build não está notarizada pela Apple, pode ser preciso tentar abri-la e depois escolher **Definições do Sistema → Privacidade e Segurança → Abrir mesmo assim**.

Se o GitHub não aceitar substituir vários ficheiros pelo navegador, abre cada ficheiro no GitHub, escolhe **Edit** e substitui o conteúdo do correspondente no ZIP. Para o ícone `.icns` e o PNG, usa **Upload files** dentro da pasta `assets`.

O ZIP descarregado dos Artifacts contém também a `.app` comprimida e `SHA256SUMS.txt`. O workflow verifica a arquitetura ARM64, a assinatura ad-hoc e o DMG. A aplicação não tem certificado Developer ID nem notarização Apple. Os artefactos expiram após 30 dias; guarda o DMG depois de o descarregar.

## Validação

Os testes do workflow verificam que os originais e os ficheiros existentes são preservados e que a exportação JPEG funciona com uma imagem RGBA. A compilação e os testes da nova interface devem ser confirmados por uma execução verde de GitHub Actions. O visual final precisa de ser visto na aplicação instalada no Mac.

## Desenvolvimento (opcional)

`cargo run` com Rust instalado. Não é necessário para obter o instalador pelo GitHub.
