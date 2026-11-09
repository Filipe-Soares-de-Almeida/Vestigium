# Changelog (pt-BR)

<!--
The Brazilian Portuguese notes of each released version, shown when the window is in Portuguese.
At release time, copy the version's section from CHANGELOG.md, keep its headings as they are
(`## [x.y.z] - YYYY-MM-DD`, `### Added`, `### Changed`, `### Fixed`, ...) and translate only the
entries. A version missing here shows its English notes.
-->

## [0.1.0] - 2026-11-09

A primeira versão.

### Added

- Um agente leve que monitora o PC a partir da bandeja, como o seu usuário e sem conexão de rede. Ele
  guarda os últimos minutos de carga, os programas que mais usam o PC e os eventos do Windows, para
  que um travamento possa ser explicado depois que passou.
- Um relatório para cada travamento, programa que parou de responder, engasgo e desligamento
  inesperado: quando aconteceu, quanto durou, a causa provável com a evidência por trás dela, o que
  tentar, o que aconteceu segundo a segundo e um gráfico do minuto anterior. O botão Copiar relatório
  coloca tudo na área de transferência em Markdown.
- Um modo sensível que verifica o PC com muito mais frequência por 30 minutos, para pegar os engasgos
  curtos que atrapalham jogos, áudio e chamadas.
- Atividade: os estados do PC desde a meia-noite com cada travamento e sobrecarga marcados, o
  histórico dos dias anteriores e o que pesa no PC agora, por programa.
- Saúde: o PC verificado por área a cada hora (discos, desligamentos inesperados, memória,
  inicialização, hardware e vídeo, proteção), com alertas que mostram se pioraram desde que abriram,
  e uma verificação completa com direitos de administrador.
- Testes: o Diagnóstico de Memória do Windows, um teste rápido de memória, o SMART ou a saúde NVMe de
  cada disco, uma verificação de superfície só de leitura, uma verificação do sistema de arquivos e um
  teste de velocidade, com as verificações anteriores de cada disco guardadas.
- Dumps: os dumps de tela azul, de reinício do driver de vídeo e de falha de programas que o Windows
  guarda, explicados no próprio PC sem internet, com o código de parada em palavras simples e o driver
  mais provável, e apagados quando você confirma.
- Capturas: a medição da latência DPC/ISR de cada driver e do tempo de quadro dos programas na tela,
  de 2 minutos a 24 horas, que aponta o driver ou programa por trás dos engasgos; cada captura fica
  num histórico.
- Otimizações: configurações do Windows que vale mudar neste PC, sugeridas só quando um sinal mostrou
  que ajudariam, cada uma aplicada e desfeita com um clique. Software de segurança, Windows Update e
  arquivo de paginação nunca são oferecidos.
- As tarefas que precisam de direitos de administrador rodam num auxiliar separado, que os pede uma
  de cada vez.
- Atualizações na janela: com a sua permissão, ela procura no GitHub uma versão mais nova no máximo
  uma vez por dia, mostra o que mudou e a instala.
- A janela em inglês e português do Brasil, clara ou escura, em quatro temas de cor, no nível de
  detalhe Simples, Detalhado ou Técnico.
- Um instalador por usuário, em inglês e português do Brasil, que não pede direitos de administrador
  e, quando você pede, apaga o histórico ao desinstalar.
