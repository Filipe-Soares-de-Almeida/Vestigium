# Changelog (pt-BR)

<!--
The Brazilian Portuguese notes of each released version, shown when the window is in Portuguese.
At release time, copy the version's section from CHANGELOG.md, keep its headings as they are
(`## [x.y.z] - YYYY-MM-DD`, `### Added`, `### Changed`, `### Fixed`, ...) and translate only the
entries. A version missing here shows its English notes.
-->

## [0.1.1] - 2026-11-11

### Added

- Faixa acima de todas as telas enquanto qualquer captura profunda roda.
- Botões para cancelar capturas, verificações de volume, leituras dos dumps e exames completos.
- Bloco O que foi medido e marca Cobertura na página de uma captura profunda.
- Selos na bandeja e avisos na janela quando o monitor não está lendo o PC.
- Status Monitor sem resposta, com os últimos valores mantidos na tela.
- Caixa de diálogo quando o monitor guardou à parte um histórico danificado.

### Changed

- Instala o Vestigium para todas as contas, em `C:\Program Files\Vestigium`.
- Mantém os quatro cartões da tela Início com a mesma altura.
- Separa os trechos sem leitura dos trechos suspensos em Estados do PC.
- Abre Dumps, Histórico, Hoje e os gráficos mais rápido com um histórico longo.
- Guarda os piores momentos de cada minuto numa captura profunda longa.
- Mostra o que uma verificação de volume marca para reparo na próxima inicialização.
- Compara o antes e o depois de um ajuste por hora monitorada.
- Põe a causa provável e a confiança dela no início do relatório copiado.
- Deixa de afirmar como fato uma causa incerta nos relatórios, em Agora e em Hoje.
- Deixa de apresentar um desligamento interrompido como queda de energia.
- Melhora o texto de Agora, Hoje, Saúde e Otimizações.
- Atualiza o README e as páginas de privacidade, segurança e perguntas frequentes.

### Fixed

- Corrige a janela, a captura profunda ou a leitura dos dumps que esperavam para sempre.
- Corrige uma segunda captura profunda que interrompia a primeira.
- Corrige a captura profunda parcial que dizia não haver quadros, drivers ou engasgos.
- Corrige os drivers e aplicativos que uma captura profunda culpava ou deixava de fora.
- Corrige relatórios que culpavam o programa ou disco errado, ou um evento próximo.
- Corrige os próximos passos genéricos nos relatórios de tela azul e de causa de disco.
- Corrige o relatório sem leituras que dizia que nada ficou sobrecarregado.
- Corrige ocorrências perdidas, contadas em dobro ou mostradas inteiras quando cortadas.
- Corrige enxurradas de eventos repetidos do Windows que lotavam ou dividiam relatórios.
- Corrige desligamentos inesperados perdidos, mal informados ou atribuídos a queda de energia.
- Corrige Hoje e Todos os eventos, que falhavam quando duas sobrecargas começavam juntas.
- Corrige Início e Saúde, que mostravam tudo certo para dados que não conseguiram ler.
- Corrige alertas da Saúde que fechavam e reabriam, ou chegavam com uma hora de atraso.
- Corrige alertas de disco falsos ou antigos, e HDs ociosos acordados para uma verificação.
- Corrige verificações com falha lidas como erros no sistema de arquivos ou inicialização limpa.
- Corrige verificações de superfície num disco ocupado, falhando ou desconectado.
- Corrige a verificação de superfície em andamento que sumia ao reabrir a janela.
- Corrige o teste de velocidade que enchia um volume pequeno ou deixava o arquivo para trás.
- Corrige o Diagnóstico de Memória do Windows que quebrava Histórico e outras telas.
- Corrige páginas de teste e de programa que ficavam vazias ou sem aviso num erro.
- Corrige ajustes sugeridos com pouca evidência ou pelos testes de disco do aplicativo.
- Corrige entradas de inicialização de antivírus oferecidas para remoção.
- Corrige Desfazer e ajustes com falha que deixavam configurações que você não escolheu.
- Mostra o que um ajuste interrompido alterou, com Desfazer quando isso é conhecido.
- Corrige o ajuste aplicado, ou a mensagem errada, quando o ponto de restauração falhava.
- Corrige Dumps, que perdia dumps em subpastas ou lia os de outra conta.
- Mostra por que alguns dumps não puderam ser apagados.
- Corrige lacunas de leitura quando disco, histórico ou contadores do Windows falham.
- Corrige leituras e eventos perdidos na suspensão ou numa escrita do histórico que falhou.
- Corrige o monitor que falhava com um histórico danificado ou indisponível por um momento.
- Corrige Apagar o histórico, que perdia dados não salvos ou deixava itens antigos.
- Corrige uma configuração de retenção inválida que redefinia as outras duas.
- Corrige o monitor que acumulava arquivos de resultado ou memória sem limite.
- Corrige o histórico apagado quando o relógio do PC é adiantado demais.
- Corrige travamentos e o gráfico de Agora afetados por uma mudança no relógio do PC.
- Corrige uma pausa ou o modo sensível que terminava na hora errada.
- Corrige a janela que congelava com o monitor ocupado ou o perdia quando ele reiniciava.
- Corrige o Vestigium aberto duas vezes, que rodava dois monitores ou fechava sem aviso.
- Corrige o ícone e o menu da bandeja que congelavam ou mostravam um estado antigo.
- Corrige o monitor que continuava rodando ao sair da conta ou durante uma instalação.
- Corrige Iniciar com o Windows apontando para outra cópia do Vestigium.
- Corrige a visualização ao vivo e o tempo sem resposta em Agora, que ficavam defasados.
- Corrige a lista Programas, que não achava programas na pesquisa ou perdia o caminho deles.
- Corrige a busca por atualizações com falha que dizia Versão mais recente instalada.

### Security

- Recusa iniciar o auxiliar de administrador de uma pasta que a conta pode alterar.
- Mantém a chave de assinatura das atualizações fora da compilação do instalador.
- Recusa um plano de ajustes trocado enquanto o pedido de administrador está aberto.
- Recusa rodar uma biblioteca deixada ao lado do auxiliar de administrador.
- Ignora links colocados dentro de uma pasta de dumps.
- Impede que um link leve o arquivo de resultado do auxiliar para outra pasta.
- Impede que uma pasta trocada por um link desvie uma tarefa do auxiliar.
- Recusa um Desfazer que o auxiliar de administrador não registrou.

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
