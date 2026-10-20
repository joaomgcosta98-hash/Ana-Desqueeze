# AnaDesqueeze · João Costa ©

Aplicação de fotografia anamórfica com interface escura para **Mac Apple Silicon (M1 ou posterior), macOS 13 ou posterior**.

**Este ZIP é um projeto de código-fonte. Ainda não contém um instalador.** O GitHub Actions cria a aplicação e o DMG num Mac do GitHub. Não é preciso instalar Rust, Xcode nem compilar no teu Mac.

## 1. Criar o repositório, pelo navegador

1. Entra na tua conta em https://github.com e abre https://github.com/new.
2. Nome: `AnaDesqueeze`. Escolhe a visibilidade. Num repositório público, o código fica público; num privado, verifica a disponibilidade e os limites de Actions do teu plano.
3. Marca **Add a README file** e carrega em **Create repository**. As instruções seguintes assumem que a branch principal se chama `main`.
4. Descompacta `AnaDesqueeze-projeto-corrigido.zip` no Finder e abre a pasta AnaDesqueeze.
5. No GitHub, em **Code → Add file → Upload files**, arrasta os ficheiros `Cargo.toml` e `README.md` e as pastas **src** e **scripts**. Arrasta o conteúdo indicado, não a pasta AnaDesqueeze nem o ZIP. Guarda com **Commit changes** diretamente em `main`.
6. Confirma que `Cargo.toml` está na página principal do repositório, e que existem `src/main.rs` e `scripts/package-macos.sh`.

## 2. Adicionar o workflow (atenção à pasta oculta)

A pasta `.github` do projeto é oculta no Finder. Para não depender do envio de pastas ocultas, usa este método:

1. No GitHub, escolhe **Add file → Create new file**.
2. No campo do nome escreve exatamente: `.github/workflows/mac-installer.yml`. O GitHub cria as pastas automaticamente.
3. No Mac, abre `workflow-para-copiar.txt`, que vem neste projeto, com o Editor de Texto. Copia todo o conteúdo e cola na área de edição do GitHub. Não copies cercas de Markdown nem alteres a indentação.
4. Guarda com **Commit changes** diretamente em `main`.
5. Confirma que o ficheiro aparece em `.github/workflows/mac-installer.yml`.

Se já tiveres enviado a pasta `.github` corretamente, podes saltar esta criação manual. Não basta enviar `workflow-para-copiar.txt` para a raiz: só o caminho `.github/workflows/mac-installer.yml` ativa o workflow.

## 3. Obter o instalador

1. Abre **Actions**. Se solicitado, ativa os workflows deste repositório.
2. A criação do workflow em `main` inicia automaticamente **Criar instalador macOS**. Se não houver execução, seleciona esse workflow e escolhe **Run workflow → main → Run workflow**.
3. Espera que a execução termine com um visto verde. A primeira compilação pode demorar vários minutos.
4. Abre a execução concluída. Na secção **Artifacts**, clica em **AnaDesqueeze-AppleSilicon**. Precisas de ter sessão iniciada no GitHub.
5. Descompacta o ZIP descarregado. Contém:
   - `AnaDesqueeze-AppleSilicon.dmg`: instalador recomendado.
   - `AnaDesqueeze-AppleSilicon-app.zip`: alternativa com a `.app` dentro, preservando permissões e estrutura.
   - `SHA256SUMS.txt`: somas de verificação.
   - `Cargo.lock`: versões exatas das dependências usadas nessa execução.

Os artefactos deste workflow expiram após 30 dias, ou antes se forem eliminados. Descarrega-os e guarda uma cópia. Podes repetir a execução para criar novos artefactos. Para repetir exatamente as mesmas dependências, adiciona o Cargo.lock descarregado à raiz do repositório antes de executar novamente.

## 4. Instalar no Mac

1. Abre `AnaDesqueeze-AppleSilicon.dmg`.
2. Arrasta **AnaDesqueeze** para **Applications / Aplicações** dentro da janela do disco.
3. Ejeta o disco e abre AnaDesqueeze na pasta Aplicações.
4. Se o macOS bloquear por ser de um programador não identificado, depois da tentativa de abertura vai a **Definições do Sistema → Privacidade e Segurança → Abrir mesmo assim** e confirma. Faz isto apenas para a compilação do teu projeto em que confias.

Alternativa: descompacta `AnaDesqueeze-AppleSilicon-app.zip` e arrasta a aplicação extraída para Aplicações.

A aplicação recebe uma assinatura ad-hoc para integridade do pacote. **Não tem certificado Developer ID nem notarização Apple**; essa assinatura não elimina os avisos de segurança do macOS.

## Se algo falhar

- **Não aparece o workflow em Actions:** confirma o caminho `.github/workflows/mac-installer.yml`, a presença na branch principal e se Actions está ativado. O botão Run workflow exige o workflow na branch predefinida.
- **Cargo.toml não encontrado:** a pasta AnaDesqueeze ficou um nível abaixo da raiz. Envia o conteúdo para a raiz, conforme o passo 1.
- **Execução vermelha:** abre o passo vermelho e copia o erro. Ainda não existe um instalador válido dessa execução. Não descarregues o projeto em Code → Download ZIP à espera de encontrar a app.
- **Sem capacidade de execução:** consulta as definições e os limites de faturação de Actions da conta. A disponibilidade depende do plano e das políticas do GitHub.
- **Sem Artifacts:** espera pelo fim; os instaladores só são publicados depois dos testes, compilação e verificações de empacotamento.

## Funcionalidades e limites

- Importação de ficheiros, pasta (primeiro nível) e arrastar ficheiros para a janela.
- Desqueeze 1.25×, 1.33×, 1.5×, 1.6×, 1.8×, 2× e personalizado entre 0.5× e 4×.
- Pré-visualização e exportação em lote para JPEG, PNG e TIFF.
- Os originais são lidos sem escrita. As exportações usam sufixo `_desqueezed` e reservam o ficheiro de destino sem sobrescrever nomes existentes.
- RAW e HEIC usam `sips` do macOS como alternativa de descodificação; o suporte depende do macOS e da câmara. Não há exportação DNG RAW.
- A exportação é renderizada. EXIF, perfis ICC e profundidade original não são garantidos. JPEG converte para RGB de 8 bits e descarta transparência.
- A pré-visualização tem resolução reduzida. A exportação usa a imagem fornecida pelo descodificador e limita a largura de saída a 100 000 píxeis.

## Estado de validação desta entrega

Foram revistos o código, o workflow e o empacotamento e verificada a sintaxe do script e do YAML. **Não foi executada compilação local nem remota nesta entrega**, porque ainda não existe um repositório GitHub associado. A interface e a abertura da app ainda precisam de ensaio num Mac.

O workflow executa testes de preservação de originais, nomes existentes e exportação JPEG, compila para `aarch64-apple-darwin`, verifica a arquitetura ARM64, o Info.plist, a assinatura ad-hoc e a integridade do DMG. Essas verificações não substituem um ensaio da interface nem a notarização Apple.

## Referências

- GitHub: https://docs.github.com/en/repositories/working-with-files/managing-files/adding-a-file-to-a-repository
- Execução manual: https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow
- Descarregar artefactos: https://docs.github.com/en/actions/how-tos/manage-workflow-runs/download-workflow-artifacts
- Apple: https://support.apple.com/en-ie/102445

## Desenvolvimento (opcional)

Com Rust e ferramentas de desenvolvimento instaladas: `cargo run`. Este passo não é necessário para a instalação descrita acima.
