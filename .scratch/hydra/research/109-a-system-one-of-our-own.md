<<<hydra-result 7cb3b7ee573a7257b9c0>>>
# Ticket 109: un System 1 propio

Todo lo externo lo leí hoy, 2026-10-04. No llamé a la API de Jev, no lancé `hydra run` y no edité nada.

## Jev hoy

**Qué ofrece.** Sigue habiendo un solo endpoint, `POST https://api.typesafe.ai/v1/systemone`, con `state`, `model` y un mapa de preguntas `noul`, `choice` o `score` ([API](https://docs.typesafe.ai/api)).

- **Modelo y precio.** El único modelo es `jev-1.13.0`. Cuesta 0,042 USD por millón de tokens de entrada, con salida gratis y 64k de contexto ([modelos](https://docs.typesafe.ai/models)).
- **Límites.** La página dice hoy 100K tokens por segundo y 80 peticiones por segundo. La investigación 81 anotó 250.000 y 1.200 por minuto el 25 de septiembre, así que cambiaron.
- **Confianza.** Ya tiene fórmula publicada. En `noul` es `|2p - 1|`, en `choice` es `(p_max - 1/n) / (1 - 1/n)` ([confidence](https://docs.typesafe.ai/confidence.md)). Es la forma de la distribución y no añade información a la probabilidad.
- **RLCD.** Sigue sin paper, sin función de recompensa y sin diagrama de fiabilidad ([dsebastien.net](https://www.dsebastien.net/reinforcement-learning-for-calibrated-decisions-rlcd/)).
- **Sin ajuste por cliente.** Typesafe no hace fine-tuning ni LoRA con datos de clientes ([modelos](https://docs.typesafe.ai/models)).

**Qué hay en el repo.** `crates/grader/src/jev.rs` tiene `build`, `patches` y `score`. `ask` no existe: el mapa 94 decidió no portarlo y el build 220 borró `jev_probe.py` (`IMPLEMENTATION.md`, líneas 3685 y 4125). Hoy nada en el repo puede llamar a Jev.

**Dónde vale, de más a menos.**

1. **Orden de lectura de diffs.** El build 134 midió AUC 0,964 y Brier 0,332 contra 0,134 de la tasa base, sobre 25 diffs. Ordena bien y calibra mal. Sirve para decidir qué diff leer primero o cuándo añadir un asiento adversario, nunca como umbral ni como voto. A unos 5.200 tokens por diff, mil diffs cuestan 0,22 USD.
2. **Generador de features.** Varias `noul` estrechas alimentan la logística de `crates/grader/src/baseline.rs`. Typesafe documenta el patrón en su [cookbook](https://docs.typesafe.ai/cookbooks/autoresearch_feature_discovery.md). El D4 del build 143 dejó a Jev fuera del modelo en ese ticket.
3. **Etiquetas del grader.** Un `choice` sobre las 7 clases de tarea sería barato, pero el texto de la tarea es texto de record y eso es export. Ya hay 521 filas clasificadas, así que aporta poco.
4. **Router del mapa 77.** No. Las features del record se clasifican en local a 0 USD.
5. **Veredictos de forma cerrada.** No. Es sustitución según el mapa 76-I, y Jev no escribe la `note`.

## OpenJev y parecidos

OpenJev existe. Ninguno de estos replica RLCD: todos leen los logits de las opciones en una pasada, y la calibración, cuando la hay, es una temperatura ajustada después.

| Nombre | Licencia | Base y tamaño | M2 de 8 GB | T4 |
|---|---|---|---|---|
| [openjev](https://github.com/TheoLeeCJ/openjev) | MIT | Qwen3.5-4B congelado, sin código de entrenamiento | no en BF16; trae backends llama.cpp y MLX y un Qwen3-0.6B de demo | sí, estimación mía |
| [mini-jev](https://github.com/r-ms/mini-jev) | MIT | Qwen3-4B, 8,5 GB de memoria | no | sí, estimación mía |
| Laya, de Convai | Apache-2.0 | ModernBERT-large 421M, y mmBERT 322M multilingüe | sí, estimación mía; hay [versión ONNX](https://huggingface.co/tozp/laya-onnx) | sí, 33 a 40 ms medidos |
| Kev-9B | Apache-2.0 | Qwen3.5 9B | no, pide un Mac de 32 GB | no lo pude comprobar |
| [parallel-constrained-decoding](https://huggingface.co/spaces/drinkmoonshine/parallel-constrained-decoding) | Apache-2.0 | Qwen2.5-1.5B a 4 bits, MLX | sí | no aplica |

- **openjev.** Su README da 0,845 de acuerdo modal contra 0,883 de Jev en 102 filas. Último commit el 2026-09-22.
- **mini-jev.** Su README avisa de que sus porcentajes no son probabilidades calibradas.
- **Laya.** Sin ajuste acierta 0,362, y el 0,766 es de un checkpoint entrenado en el split del propio benchmark. Una temperatura por tipo de pregunta baja el ECE de 0,466 a 0,081 ([aiweekly](https://aiweekly.co/alerts/convai-ships-laya-a-421m-modernbert-decision-model-apache-20)). Es una base para especializar.
- **Kev-9B.** Solo lo vi en una [nota del 23 de septiembre](https://rohitraj.tech/notes/jev-alternatives-open-weights-decision-models-2026) que no da su enlace.

Los datos de mini-jev y parallel-constrained-decoding salen de sus repos y de la [lista de Apidog](https://apidog.com/vi/blog/openjev-open-source-jev-alternatives), que no trae fecha.

## Nuestro propio System 1

Hydra ya tiene uno pequeño. `baseline.rs` es una logística sobre features del record con modelos `outcome` y `verdict`, y escribe `scores.jsonl` en cada collect. Lo que falta son etiquetas. `needs_correction` tiene 25 hechas a mano y 5 veredictos humanos. La clase de tarea tiene 521.

- **Cabeza sobre un encoder congelado.** Es lo que haría. Se sacan embeddings del diff y del texto de la tarea, se entrena una logística con log loss y se ajusta una temperatura en un tramo posterior en el tiempo. Entrena en el Mac en minutos y nada sale de la máquina. Respondería clase de tarea, grado, outcome y, cuando haya etiquetas, `needs_correction`.
- **RLCD.** No se puede copiar porque no está publicado. Lo aprovechable ya lo dijo el mapa 81: regla de puntuación propia y temperatura.
- **Destilar de Jev.** Jev ordena bien nuestros diffs, y sus respuestas sobre diffs ya commiteados servirían de etiquetas blandas por céntimos. Antes hay que leer el [Master Customer Agreement](https://typesafe.ai/legal/mca), que no leí, por si prohíbe entrenar con sus salidas.
- **JEPA.** Entrena un encoder para predecir el embedding de una vista a partir de otra, con la pérdida en el espacio de embeddings. Produce representaciones. No da tipos ni probabilidades, así que la cabeza y la calibración harían falta igual. [LLM-JEPA](https://arxiv.org/abs/2509.14252), de Huang, LeCun y Balestriero, lo usa como pérdida auxiliar al afinar un LLM con pares de vistas, y sus resultados son sobre datasets de miles de pares. Aquí las vistas naturales serían tarea y diff, o ticket y Outcome. Con 350 a 550 ejemplos no aprende nada que un encoder preentrenado no traiga ya, y exige GPU alquilada. Solo lo probaría como pérdida extra el día que se haga el LoRA de Gemma 4B de la fase 3.

## Una pregunta sobre otra repo

[withheld: details of a private repository]

## Qué haría primero

1. **Ticket de build: el conjunto de diffs originales.** Un comando que une cada veredicto humano de la cola del build 143 con el diff tal como lo devolvió el agente, sacado de la rama del run, y escribe un jsonl local. Tests sobre repos sintéticos, porque un run no puede leer `~/hydra/runs/`. Es el cuello de botella de todo lo demás.
2. **Ticket de build: System 1 v0.** Embeddings desde el Ollama que ya usa el aprendiz, para no añadir dependencias, y una cabeza logística con temperatura sobre `baseline.rs`. Se mide con Brier y AUC contra la tasa base por proveedor y contra las 25 respuestas de Jev del build 134. No comprobé hoy qué modelos de embeddings sirve Ollama.

## Pendiente de mí

- **Jev sobre diffs sin commitear.** El commit 839b76f limita la allowlist a este repo con destino a tu proyecto de GCP, y Typesafe no está ahí. Además, revivir `jev ask` revierte el mapa 94. Las dos son decisiones tuyas y van al mapa.
- **Destilación.** Leer el MCA de Typesafe antes de entrenar con sus respuestas.
- **El ticket.** La sesión tiene que escribir el `## Answer` del 109. Yo no edité nada.

## Cambiado

Nada. Ningún archivo editado, ningún commit, ninguna llamada a Jev.

## Encontrado

- Los límites de Jev cambiaron respecto a la investigación 81.
- `IMPLEMENTATION.md`, línea 698, dice que `config/export-allowlist.json` no existe. Existe desde el commit 839b76f.
- Las fuentes secundarias no coinciden en la cifra de Jev frente a Laya. Una da 0,857 y otra 0,727. No abrí la model card.
- El ticket 109 y el 81 escriben "Diego Almeida". La investigación 81 dice Diogo.
- No pude comprobar tres cosas: las 350 líneas del export, que viven fuera del worktree, los créditos gratis de Jev y la latencia actual.
- El README del aprendiz dice que sus pares no entran en `hydra export`, así que el conjunto del mapa 87 no puede salir hoy hacia ningún entrenamiento.
<<<end hydra-result 7cb3b7ee573a7257b9c0>>>