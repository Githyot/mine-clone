# Resumo do projeto: jogo estilo Minecraft em Rust

Documento único para avaliação. Cada seção aponta para o arquivo detalhado na pasta `plan/`. Situação em 16/10/2026:
**plano completo, 7 músicas prontas, nenhum código ainda** (falta criar o repositório no GitHub).

---

## 1. Objetivo
Um jogo com tudo que o Minecraft Java tem (versão de referência 26.2, de junho de 2026), escrito em **Rust**, muito mais rápido, com
mods reescritos em Rust e ideias do Terraria. Nada de Java. Nenhum código, textura ou música da Mojang é copiado; usamos só o
comportamento público (Minecraft Wiki, changelogs) e projetos abertos.

## 2. Inventário do Minecraft (`inventory/`)
| Arquivo | Conteúdo |
|---|---|
| `blocks.md` | ~1.100 blocos agrupados em famílias (madeiras × 12 espécies, pedras, 16 cores, cobre, redstone...) |
| `items.md` | ferramentas (7 materiais), armaduras, materiais, comida, poções, discos, cacos de cerâmica |
| `mobs.md` | ~85 mobs + ~45 entidades + os blocos de IA compartilhados por todos |
| `world.md` | 3 dimensões, 66 biomas, pipeline de geração, estruturas |
| `systems.md` | combate, crafting, encantamento, poções, aldeões, redstone, comandos, multiplayer, menus |

Pendente: o Minecraft 26.3 já saiu e o conteúdo dele ainda não foi listado.

## 3. Motor e desempenho (`architecture.md`)
- **Rust + wgpu sobre Vulkan** (Metal no Mac, DirectX 12 de reserva). Pode trocar para Vulkan puro sem mexer na jogabilidade.
- As 5 técnicas do vídeo "262.000 vezes mais rápido": culling, otimização de compilação, LOD, greedy meshing, multithread.
- Desenho na GPU com poucas chamadas por quadro, chunks 16×16×16 com paleta, iluminação em paralelo, redstone em grafo,
  servidor dividido em regiões que rodam em núcleos diferentes.
- Metas (ainda não medidas, são objetivos): 240+ FPS com 32 chunks de distância, 128+ chunks com LOD, 1.000+ chunks gerados por
  segundo, tick do servidor abaixo de 5 ms, abertura do jogo em menos de 3 s. Testes automáticos barram mudanças que deixem o jogo mais lento.
- Singleplayer roda um servidor interno, então o multiplayer usa o mesmo código.

## 4. Roteiro (`roadmap.md`)
| Fase | Entrega jogável |
|---|---|
| 0 | Motor: janela, renderização, chunks, threads, medições |
| 1 | Modo criativo: terreno, construir, luz, salvar |
| 2 | Sobrevivência: vida, fome, crafting, primeiros mobs, minérios 1–6, música |
| 3 | Overworld completo: todos os biomas, estruturas, aldeões, fazendas, clima |
| 4 | Redstone e automação |
| 5 | Nether e End, chefes clássicos |
| 6 | Encantamento, poções, magia, acessórios, chefes novos, NPCs em casas, modo difícil |
| 7 | Multiplayer e servidor grande, comandos |
| 8 | Conteúdo completo, arte, som, acessibilidade → versão 1.0 |

Em paralelo, a trilha de mods M1–M5: dados → ganchos de eventos → scripts WASM → primeiros mods portados → mods no servidor.

## 5. Mods (`modding.md` e `mods/`)
Regra principal: **o jogo base é carregado como um mod**, então mods usam o mesmo caminho. Três formas de adicionar um mod:
arquivos de dados, scripts WASM (seguros), ou código Rust nativo.

**41 mods analisados**, cada um com licença, o que entra e como fica mais rápido:

| Tipo | Mods |
|---|---|
| Viram parte do jogo base | JEI, Mouse Tweaks, Chat Heads, FastFurnace, Clean Swing Through Grass, Corpse, LambDynamicLights, Immersive UI, Presence Footsteps, 3D Skin Layers, Essential (hospedar mundo), What Are They Up To |
| Magia e progressão | Iron's Spells, Ars Nouveau, Apotheosis (todos dentro do nosso sistema de magia) |
| Mobs e animais | Alex's Mobs, Naturalist, Mutant Monsters, Guard Villagers, Goblin Traders, Ecologics |
| Estruturas e vilas | Towns and Towers, CTOV, YUNG's Better Dungeons, Moog's ×2, Explorify, When Dungeons Arise |
| Decoração | Macaw's ×9, Handcrafted, Sophisticated Backpacks |
| Mundo e clima | Serene Seasons, Snow! Real Magic!, Particle Rain, Falling Tree, Continuity |
| Dimensões e veículos | The Aether, Deeper and Darker, Immersive Aircraft, Aquaculture |
| Combate e animação | Better Combat, Fresh Animations |

**Licenças:**
- **Podemos usar o código** (MIT, BSD, MPL), com crédito: JEI, Mouse Tweaks, FastFurnace, Snow! Real Magic!, Particle Rain, Goblin Traders, Presence Footsteps, Chat Heads, e parte do Macaw's.
- **Copyleft** (LGPL, GPL, AGPL): Alex's Mobs, Deeper and Darker, Aether, YUNG's, Moog's, Falling Tree, Immersive Aircraft, Mutant Monsters, Continuity. O código só poderia entrar num pacote separado e aberto; o jogo base usa só as ideias.
- **Todos os outros:** só as ideias, com código e arte nossos.

## 6. Ideias do Terraria (`magic-and-rpg.md`, `terraria-content.md`)
- **Magia:** barra de mana, varinhas, cajados, livros de magia com pergaminhos, magias montadas com glifos, 8 escolas.
- **Combate:** barras de vida sobre os mobs, barra de chefe com fases, acessórios (pulo duplo, dash, asas), 4 classes pelo equipamento, modificadores em itens.
- **Chefes:** escada de progressão (item de invocação → chefe → material do próximo nível), bolsa de tesouro por jogador.
- **NPCs em casas** construídas pelo jogador, com condições para se mudar e felicidade.
- **Minérios:** 14 níveis, ~22 minérios novos, 7 gemas, pares sorteados por mundo, e o "World Breaker" que libera o modo difícil sem precisar de mundo novo.
- **Itens:** ~1.000 itens novos.
- Nomes inventados pelo Terraria ganham nomes nossos.

## 7. Animação e sensação de jogo (`game-feel.md`)
Ideias do vídeo de mods de animação, já no jogo base: emotes, comida diminuindo enquanto você come, itens que quicam,
corações animados, telas com transição suave, mãos que alcançam portas. Tudo pode ser desligado em "reduzir movimento".

## 8. Música (`music.md`, `assets/music/`)
- Toca como no Minecraft antigo: rara, aleatória, calma, com 10 a 20 minutos de silêncio entre as músicas.
- **7 músicas originais já feitas**, em `.ogg`: Primeira Luz e Musgo (mundo), Água Parada (debaixo d'água), Bloco a Bloco
  (criativo), Brasa (Nether), Vazio (End), Início (menu). Foram geradas por programa (`compose.py`), então podem ser refeitas.
- Opção de usar **a sua própria pasta de músicas**: só a sua, só a do jogo, ou as duas misturadas, por situação.
- As músicas originais do Minecraft não podem ser incluídas (direitos autorais do C418).

## 9. Texturas (`textures.md`)
- **Estilo do Minecraft antigo:** 16×16 pixels, pixels nítidos, paletas curtas, minérios em pontinhos. A arte é feita por nós, sem copiar nem decalcar.
- **Texture packs como no Minecraft:** pasta de pacotes, empilhar por prioridade, recarregar sem reiniciar (F3+T).
- **Importador de packs do Minecraft:** qualquer pack que você tenha funciona no seu computador. Nós só não distribuímos os arquivos da Mojang.

## 10. Decisões tomadas
- Não usamos o código vazado do Minecraft.
- Não incluímos músicas nem texturas da Mojang.
- O modo de música padrão é o "clássico".
- A magia vem ligada por padrão, mas pode ser desligada por mundo.

## 11. O que preciso de você
1. **Criar um repositório vazio no GitHub e me dizer o nome.** Sem isso não começo o código (Fase 0).
2. Ouvir as músicas e dizer se o estilo está bom (mais lento, mais melodia, mais instrumentos...).
3. Confirmar qual licença Creative Commons o Towns and Towers usa, se quiser aproveitar as construções deles.
