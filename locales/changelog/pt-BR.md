# Changelog (pt-BR)

<!--
The Brazilian Portuguese notes of each released version, shown when the window is in Portuguese.
At release time, copy the version's section from CHANGELOG.md, keep its headings as they are
(`## [x.y.z] - YYYY-MM-DD`, `### Added`, `### Changed`, `### Fixed`, ...) and translate only the
entries. A version missing here shows its English notes.
-->

## [0.1.1] - 2026-11-11

### Added

- Uma captura profunda que já está em andamento agora aparece, não importa quem a iniciou. Uma faixa
  neutra acima da tela, em todas as telas, diz "Captura em andamento" com o relógio e uma barra
  quando esta janela a iniciou e, para uma captura iniciada antes de a janela ser reaberta, por
  outra janela ou por outra conta do Windows, desde quando ela roda, quando isso é conhecido, e que
  o fim não é conhecido. Enquanto isso, o item Capturas do painel lateral e o bloco dele na tela
  Início mostram uma marca informativa. Quando uma captura iniciada em outro lugar termina, a faixa
  oferece o resultado.
- Uma captura profunda, uma verificação de volume, uma leitura dos dumps e um exame completo agora
  podem ser cancelados no próprio cartão. Cancelar uma captura ou uma verificação de volume pede
  confirmação antes e diz o que acontece: o auxiliar é avisado para parar e a janela deixa de
  esperar por ele, o tempo já capturado não é salvo, e uma verificação procura problemas sem reparar
  nada agora. A leitura dos dumps e o exame completo, que levam segundos, são cancelados na hora.
  Uma captura cancelada não guarda nada, e o cartão mantém uma linha dizendo o que foi cancelado e o
  que continua válido.
- Quando o auxiliar não termina a tempo, uma caixa de diálogo na própria tela da tarefa agora avisa
  uma vez e oferece tentar de novo. Em outra tela, o item do painel lateral e o bloco do grupo na
  tela Início recebem a marca de atenção até que esse diálogo seja visto, e o cartão mantém uma
  linha sobre a última tentativa. Uma captura pela qual a janela deixou de esperar não afirma que
  nada foi perdido, já que o auxiliar ainda pode terminá-la.
- O auxiliar que executa uma captura profunda agora pode receber a ordem de parar. Um arquivo de
  parada ao lado da captura a encerra em até um quarto de segundo: o rastreamento e o leitor de
  quadros são fechados, nada é salvo e o auxiliar sai com um código próprio de captura cancelada.
- O ícone da bandeja e a dica dele agora avisam quando o monitor não está lendo o PC. Sem nenhuma
  leitura por mais de alguns segundos e sem nada pausado, o ícone ganha um selo de traço e a dica
  diz "sem leitura desde" a hora da última leitura; quando só algumas fontes de dados falham, ganha
  um ponto de exclamação e nomeia a fonte, ou quantas são; e quando a verificação de saúde de hora
  em hora não termina há três horas, ganha um ponto, para que nada que não foi lido pareça normal.
  Esses selos têm prioridade sobre os selos de travamentos graves e de lentidões das últimas 24
  horas, que a dica continua listando, e ficam abaixo de pausado e do modo sensível. O ícone confere
  a cada 15 segundos, então uma coleta parada aparece em até um minuto. Uma dica que passaria dos
  127 caracteres que o Windows mostra agora deixa de fora a última linha em vez de ser cortada.
- A janela agora avisa quando o monitor não está lendo. Sem nenhuma leitura por mais de alguns
  segundos e sem nada pausado, um aviso acima da tela diz "Sem leitura" com desde quando e há quanto
  tempo, e a linha no pé do painel lateral e na tela Início diz "Sem leitura desde" a hora em vez de
  "Monitorando". Quando só algumas fontes de dados falham, o aviso diz "Monitoramento parcial" e
  nomeia a fonte, ou quantas são, e a linha de status diz "Leitura parcial". Quando a verificação de
  saúde de hora em hora não termina há três horas, a linha de status diz "Monitorando · saúde não
  verificada" e a tela Saúde mostra um aviso com a hora da última verificação que terminou. Os
  avisos têm cor neutra, para não parecerem um alerta de travamento, e no nível Técnico acrescentam
  o que o Windows informou sobre cada fonte que falha.
- A janela agora distingue um monitor que está rodando, mas não responde, de um monitor de outra
  versão. Quando o monitor não responde em três segundos, a linha de status diz "Monitor sem
  resposta" e o histórico em disco continua aparecendo; a janela não diz mais que os dois são
  versões diferentes, o que fazia para qualquer resposta que falhasse. Não há aviso: na tela Agora,
  na lista de programas e na página de um programa, as partes ao vivo ficam onde estão, esmaecidas,
  com os últimos valores recebidos, e cada uma diz "Sem resposta do monitor desde" a hora do último
  valor. "Programas sem resposta" diz que não dá para saber, em vez de "Nenhum programa sem
  resposta", e o gráfico diz "Sem resposta" onde dizia "Ao vivo". Uma visualização ao vivo que perde
  o monitor avisa na hora, em vez de dizer "parado", e volta sozinha quando o monitor responde. A
  janela pergunta de novo a cada cinco segundos e se recupera sozinha; uma consulta que ainda está
  esperando nunca é repetida, então um monitor travado não deixa mais uma thread bloqueada a cada
  consulta.
- A janela agora avisa uma vez quando o monitor substituiu um arquivo de histórico danificado. Na
  primeira vez que a janela abre depois disso, uma caixa de diálogo diz que o arquivo foi guardado à
  parte e com qual nome, mostra se os ajustes aplicados, as configurações e os dados do PC foram
  mantidos no novo histórico, diz que os ajustes já aplicados continuam no PC, mas não podem mais
  ser desfeitos quando não foram mantidos, e oferece abrir a pasta. Entendi ou Escape fecha o
  diálogo, e ele só aparece de novo numa recuperação posterior com outro arquivo. Um monitor que
  reinicia antes de a janela abrir ainda avisa a janela, enquanto o arquivo guardado à parte estiver
  na pasta de dados. A janela e o monitor precisam ser atualizados juntos, porque o protocolo de
  comunicação entre eles mudou.

### Changed

- Os quatro cartões no topo da tela Início mantêm a mesma altura. Uma linha mais longa, como vários
  alertas ou uma verificação de saúde que não rodou, alarga o cartão e desloca os cartões ao lado,
  em vez de deixá-lo mais alto.
- A verificação de volume agora diz o que deixa pendente. Quando encontra problemas, o chkdsk marca
  o volume para reparo e o Windows o repara na próxima inicialização, o que pode remover arquivos
  que ele não consegue recuperar. O cartão do volume passa então a mostrar um painel com uma marca
  de atenção que avisa isso e lembra de fazer backup do que importa. O painel diz se a verificação
  marcou o volume, se o encontrou já marcado ou se ele continua marcado de antes, e fica até o
  Windows reiniciar depois da verificação. Uma verificação sem problemas deixa o cartão como estava,
  e a verificação em andamento diz que procura problemas e não repara nada agora. O README, a wiki,
  a explicação da Saúde sobre erros no sistema de arquivos, o diálogo para parar a verificação e o
  passo de reparo sugerido não dizem mais que a verificação não altera nada.
- A tela Capturas mantém o formulário enquanto uma captura roda, com Iniciar captura indisponível e
  uma linha dizendo que fica disponível quando a captura terminar. O relógio, a barra, o botão de
  cancelar e o aviso de que uma captura terminou saíram daquele cartão e foram para a faixa acima da
  tela, que agora serve para capturas de qualquer origem.
- O Vestigium agora é instalado para todas as contas do PC, em C:\Program Files\Vestigium, em vez de
  para um só usuário. Instalar, atualizar e desinstalar pedem ao Windows a permissão de um
  administrador; numa conta que não é de administrador, o Windows pede a senha de um administrador.
  O monitor e a janela continuam rodando como o seu usuário, e o histórico e as configurações ficam
  na pasta de cada conta. Instalar ou atualizar por cima da versão 0.1.0 remove aquela cópia por
  usuário (os programas, os atalhos no menu Iniciar e na área de trabalho e a entrada na lista de
  aplicativos instalados), mantém o histórico e faz uma entrada de Iniciar com o Windows apontar
  para o novo monitor. Desinstalar com a opção Remover dados do programa marcada apaga as pastas da
  conta conectada.
- A política de segurança e a página da wiki sobre os pedidos de administrador (Administrator
  prompts) agora dizem contra o que o auxiliar de administrador protege: um programa rodando numa
  conta padrão, quando a senha de administrador de outra conta é digitada no pedido de permissão,
  está coberto. Também listam o que não está: versões sem assinatura, cujo pedido de permissão
  mostra um editor desconhecido, e a cópia por usuário da versão 0.1.0 que uma conta mantém até
  atualizar.
- Uma captura profunda agora guarda o que mediu só por minuto: para cada driver, as três rotinas
  mais longas de 1 ms ou mais; para cada aplicativo, os três piores quadros lentos; e os três piores
  atrasos da sonda do monitor, em cada minuto. A página da captura mostra uma lista limitada tirada
  deles (as 50 rotinas mais longas, as cinco mais longas de cada driver e os 200 piores quadros
  lentos), e a consulta em torno de uma ocorrência lê todos eles, então uma captura longa não
  favorece mais a sua pior hora. Um driver que nunca rodou uma rotina de 1 ms ou mais agora não
  mostra nenhuma chamada individual na página dele; os totais não mudam.
- O relatório copiado em Markdown agora começa com a causa provável e a confiança dela, e mantém as
  ressalvas que a tela mostra: um intervalo incompleto, lacunas enquanto o PC não respondia, um
  teste do Vestigium rodando no momento e uma captura profunda que cobriu aqueles minutos. O
  resultado de captura copiado mostra a confiança ao lado do título.
- O resumo do relatório e o título da captura não afirmam mais uma causa como fato quando a análise
  não tem certeza: um driver ocupando um núcleo, renderização lenta, CPU no limite, memória cheia ou
  limite de energia ou de temperatura agora aparecem com "pode ter" na confiança baixa, e também na
  média para essas causas, em inglês e em português.
- A linha de Agora e a linha de causa de Hoje que nomeiam o processo que usa o disco agora dizem
  "provavelmente", como o resumo da ocorrência já fazia, porque a medida de disco por processo
  também conta dados que nunca chegam ao disco, como o tráfego de pipes e entre processos.
- A tela Dumps agora lê do histórico só os eventos de tela azul do último ano, em vez de todos os
  eventos guardados, então abre mais rápido num PC com um histórico longo e movimentado.
- As telas Histórico e Hoje não baixam mais o texto de todos os eventos do Windows guardados no
  histórico a cada atualização, o que dava cerca de 2 MB a cada cinco minutos para 90 dias de um PC
  movimentado; a página do evento continua mostrando todos os detalhes. O calendário do Histórico
  também monta as contagens de cada dia numa só passada, então um histórico com dezenas de milhares
  de travamentos não deixa mais a tela parada.
- Enquanto a janela está minimizada, ela não pede mais nada ao monitor: a consulta de status a cada
  cinco segundos e as atualizações de hoje, saúde, dumps, otimizações, eventos, memória e histórico
  param, e cada uma volta a ler assim que a janela é restaurada.
- As telas Testes, Capturas e Dumps carregam na primeira vez que são abertas em vez de na
  inicialização, o que deixa o script que a janela carrega ao iniciar cerca de 8% menor.
- A wiki não lista mais uma queda de energia durante o desligamento entre os tipos de título que um
  relatório de desligamento inesperado pode ter, já que um desligamento que não terminou não gera
  relatório, e o texto da janela para esse tipo foi removido. A página de privacidade agora diz que
  um arquivo de resultado que o monitor não consegue ler fica na pasta de dados.
- O README, a página de privacidade e a página de perguntas frequentes não dizem mais que a busca
  por atualizações é a única requisição de rede que o aplicativo faz: agora também dizem que o
  instalador baixa o runtime do WebView2 quando ele falta e que uma atualização baixa o instalador
  dela depois que você confirma.
- A linha de Hoje em inglês para uma causa de armazenamento agora diz "likely cause: the C: disk and
  its controller", como a linha em português e o relatório já diziam.
- O texto da Saúde para um desligamento interrompido não afirma mais uma falta de energia como fato:
  agora diz que o PC parou antes de terminar de desligar e lista como causas possíveis uma queda de
  energia, um travamento ou algo segurando o desligamento, já que só o evento não permite distinguir
  entre elas.
- A linha de Agora em português para uma causa de disco agora diz "provavelmente quase todo por" o
  processo, em vez do desajeitado "provavelmente principalmente por".
- A leitura do histórico para um gráfico e a limpeza de hora em hora do histórico antigo não
  percorrem mais todas as linhas guardadas mais antigas que o período pedido, então continuam
  rápidas à medida que o histórico guardado cresce, em vez de ficarem mais lentas com ele. O banco
  de dados do histórico passa para uma nova versão, então a janela e o monitor precisam ser
  atualizados juntos.
- O monitor agora informa à janela como está o próprio monitoramento: quais fontes de dados falham e
  por quê, se o PC está em suspensão, quando o monitoramento foi retomado pela última vez e quando
  rodou a última verificação de saúde e quais verificações ela não conseguiu ler. A janela e o
  monitor precisam ser atualizados juntos, porque o protocolo de comunicação entre eles mudou.
- A linha do tempo Estados do PC em Hoje agora distingue um trecho sem leitura de um trecho
  suspenso. Um trecho em que nada foi lido e o monitor não registrou pausa, suspensão, parada ou
  falha é desenhado numa quinta linha, "Sem leitura", como barras vazadas e tracejadas, e contado na
  legenda e nos totais; antes, era desenhado como "Suspenso", como se o PC estivesse em suspensão. O
  tempo antes da primeira leitura já guardada e os trechos dentro de uma pausa, suspensão, parada ou
  falha registrada continuam como suspensos. Os seis minutos mais recentes ficam vazios quando nada
  foi lido neles, porque o monitor salva o que explica uma pausa até cinco minutos depois, e um
  monitor pausado ou parado mostra o trecho em aberto como suspenso.

### Fixed

- Não é mais possível iniciar uma segunda captura profunda enquanto outra está em andamento. A
  janela agora avisa antes de pedir permissão ao Windows e, quando outra janela inicia uma no mesmo
  instante, a faixa diz que a segunda captura não foi iniciada, em vez de informar uma falha do
  auxiliar.
- A janela não espera mais para sempre pelo auxiliar elevado. Uma captura profunda que ainda roda
  dois minutos depois da duração prevista, um exame completo ou uma leitura dos dumps depois de 30
  minutos e um lote de ajustes depois de 10 minutos deixam de ser esperados, e o cartão fica livre
  para uma nova tentativa, em vez de ficar em "Capturando" ou "Lendo". Uma verificação de volume ou
  uma verificação de disco que foi parada e cujo auxiliar não respondeu depois de 30 segundos também
  deixa de ser esperada, então Parar sempre encerra a espera.
- Em português, um ajuste que não mudou nada, ou só parte das configurações, agora diz "alterado" em
  vez de "gravado", palavra que o aplicativo reserva para a escrita de dados.
- O monitor não deixa mais um buraco nas leituras enquanto o banco de dados do histórico está
  ocupado: os lotes agora são salvos por uma thread própria, então uma escrita que espera pelo banco
  de dados não atrasa mais a leitura do segundo seguinte. Apagar o histórico também descarta o que
  ainda esperava para ser salvo, então um lote enfileirado logo antes de apagar não traz mais dados
  antigos de volta depois.
- Um travamento, um atraso, a análise pós-morte de um desligamento inesperado e os dados dos discos
  do PC agora são guardados por essa mesma thread, então uma escrita lenta não atrasa mais a leitura
  do segundo em que acontece. O que o monitor lê sobre o último desligamento inesperado vem por uma
  conexão somente leitura que nunca espera por uma escrita. Uma ocorrência que não pode ser guardada
  continua sendo tentada de novo a cada segundo e é abandonada depois de cinco tentativas com falha,
  como antes.
- A limpeza de hora em hora do histórico que passou do período de retenção e a listagem dos dumps
  agora também rodam na thread de armazenamento, então nenhuma delas pode atrasar uma leitura
  enquanto percorre um histórico grande ou uma pasta lenta.
- Um disco que leva segundos para responder à verificação de saúde não deixa mais um buraco nas
  leituras. As leituras de disco da rodada de verificação de saúde agora rodam numa thread própria,
  e os alertas que resultam delas são guardados pela thread de armazenamento, então um disco ocupado
  ou falhando atrasa só a própria rodada de verificação. Um segundo pedido de rodada enquanto outra
  ainda está lendo substitui o que está esperando. O monitor continua lendo justamente nos momentos
  em que o disco está lento, que é quando o relatório de um travamento precisa dessa evidência.
- Depois que o dump de uma tela azul é lido, a página do relatório e o relatório copiado usam os
  próximos passos do próprio dump, em vez da atualização genérica de driver. Quando o dump culpa um
  componente do Windows que atende outros drivers, como o storport.sys, a tela Dumps não diz mais
  para atualizar esse componente: ela aponta o driver da controladora de disco para armazenamento e
  os drivers do chipset e de rede para o resto.
- O relatório de uma tela azul não manda mais, por padrão, atualizar os drivers de vídeo e do
  chipset. Uma tela azul com código de parada de hardware agora pede para desfazer overclocks,
  verificar as temperaturas e rodar o diagnóstico de memória, e qualquer outra tela azul pede para
  abrir Dumps e ler o dump. Os relatórios já guardados mantêm o passo antigo.
- Um relatório cujo intervalo não tem leituras não diz mais que nenhum recurso ficou sobrecarregado:
  diz que o Vestigium não tem medições daquele momento. Quando há leituras, mas nada passou de um
  limite, diz que nenhum dos limites que o Vestigium usa foi ultrapassado e lista a GPU e a
  atividade de disco quando estavam altas. Para isso, a janela e o monitor precisam ser atualizados
  juntos.
- Um disco NVMe que só informou uma temperatura fora da faixa segura não aparece mais como um alerta
  crítico que manda fazer backup e trocar o disco. Agora ele abre um alerta de atenção, "O disco
  esquentou demais", que diz que o disco não está falhando e aponta as saídas de ar e as ventoinhas,
  e o alerta fecha quando o disco esfria. Os outros avisos críticos de um disco NVMe continuam
  críticos.
- Um arquivo de resultado que o monitor não consegue ler, como um que um auxiliar mais novo salvou
  num formato que ele não conhece, agora é apagado da pasta de dados depois de ficar 30 dias sem
  mudar, em vez de ficar lá para sempre, e a página de privacidade diz isso.
- Enquanto o monitor está travado ou ocupado, as medições de tempo e os eventos do Windows que
  esperam por ele agora têm um teto, em vez de crescerem na memória sem limite. Um evento do Windows
  que não coube é informado e lido de novo na próxima inicialização.
- Reiniciar o monitor até cinco minutos depois de um desligamento inesperado não perde mais esse
  desligamento: o que a caixa-preta guardava é mantido até o Windows confirmar a queda ou a espera
  acabar.
- Pausar o monitoramento bem quando uma pausa programada anterior termina não deixa mais a pausa
  antiga encerrar a nova: a pausa mostrada e a pausa em vigor continuam sendo a mesma.
- Sair da conta, desligar o PC ou um pedido do instalador para fechar o monitor agora também encerra
  o processo do monitor, em vez de deixá-lo rodando sem monitorar.
- O monitor agora tenta de novo a cada minuto ler o fluxo de eventos do Windows quando não conseguiu
  iniciá-lo, e recomeça do início dos eventos retidos quando o Windows recusa a posição salva. Um
  evento que ele não consegue ler é informado em vez de pulado, e a posição salva fica antes dele,
  para que a próxima inicialização o leia de novo.
- Quando Apagar o histórico falha, por exemplo porque o banco de dados está ocupado, o monitor não
  descarta mais os minutos de leituras e eventos que ainda guarda na memória, e nada registrado
  antes de um apagamento bem-sucedido é salvo depois dele. A caixa-preta, a leitura diária do SMART
  e o gráfico ao vivo recomeçam juntos depois de apagar o histórico.
- Depois de apagar o histórico, as telas Memória, Discos, detalhe de teste, Capturas e Saúde, e as
  contagens no painel de navegação, não continuam mais mostrando os testes, capturas, alertas e
  eventos apagados até a janela ser reaberta.
- Clicar em Cancelar num teste ou numa verificação de disco em andamento agora mostra o motivo numa
  mensagem quando o pedido de parada falha, em vez de não fazer nada.
- A página de detalhe de um teste ou do Diagnóstico de Memória do Windows cuja leitura dos testes
  falha agora mostra o erro com um botão Tentar novamente, em vez de uma página vazia.
- O tempo sem resposta de uma janela na tela Agora passa a recomeçar quando o mesmo programa trava
  uma segunda vez enquanto a visualização ao vivo estava parada, em vez de contar a partir do
  primeiro travamento.
- Uma busca por atualizações que o canal de versões responde com um erro, como um problema no
  servidor ou um limite de requisições, agora aparece como falha em vez de "Versão mais recente
  instalada", e a busca automática tenta de novo na próxima vez que a janela abre, em vez de esperar
  um dia.
- Passar rápido entre telas que mostram a visualização ao vivo não pode mais interromper o fluxo ao
  vivo da nova tela, que ficava congelada no primeiro segundo.
- A página de um programa cuja leitura dos eventos de hoje falha agora mostra o erro com um botão
  Tentar novamente, em vez de dizer que o programa não teve eventos hoje.
- Uma verificação do sistema de arquivos cuja execução do chkdsk termina com o código de saída 3,
  que o Windows usa para "não foi possível verificar" (acesso negado, ou um volume que não pode ser
  aberto), agora sempre aparece como uma verificação com falha. Antes, um volume que por acaso
  estava marcado como sujo fazia o resultado aparecer como erros no sistema de arquivos.
- Uma entrada de inicialização ou etapa do boot cujo nome escreve "Anti-Malware" ou "Anti-Virus" com
  hífen agora é reconhecida como produto de segurança, então nunca é oferecida para remoção.
- Um disco do sistema ocupado durante um dos testes de disco do próprio aplicativo não conta mais
  como motivo para sugerir desativar o Pré-carregamento de programas (SysMain), e não aparece mais
  como disco ocupado na comparação de antes e depois dessa alteração.
- A Economia de energia do PCI Express não é mais sugerida só por causa de tempo alto de driver.
  Agora ela exige uma captura profunda que tenha apontado um driver de disco ou de rede, e a
  explicação dela diz isso.
- O alerta "Falhas acima do normal nesta semana" não fecha e reabre mais conforme a contagem semanal
  de falhas e travamentos oscila em torno do limite. Depois de aberto, ele fica aberto até a semana
  cair abaixo de duas falhas ou abaixo de uma vez e meia a média semanal das semanas anteriores,
  então o histórico dele tem uma entrada em vez de várias.
- Um SSD não recebe mais um alerta de atenção pelos contadores SMART que só indicam setor falhando
  num HD, e um bloco SMART com checksum errado, como alguns adaptadores USB devolvem, é ignorado em
  vez de ser lido como atributos.
- Um disco ocupado durante uma verificação de superfície, uma verificação de volume ou um teste de
  velocidade desse mesmo disco agora é reconhecido como carga do teste, qualquer que seja o programa
  que o Windows aponte como causa, como o processo do chkdsk que a verificação de volume inicia. O
  histórico marca isso como causado por um teste, em vez de sugerir os passos comuns para disco.
- Quando dois ou mais programas ficam sem resposta juntos numa mesma ocorrência, cada um agora é um
  suspeito de confiança média, então uma causa do sistema que foi medida, como falta de memória,
  aparece antes deles. Um programa que ficou sem resposta duas vezes numa ocorrência é citado uma
  vez só, com a maior das duas confianças.
- O relatório depois de um desligamento inesperado e as verificações de saúde que leem os eventos
  guardados agora julgam de novo cada evento guardado com as regras da versão instalada. Um evento
  salvo por outra versão que as regras atuais não reconhecem mais deixa de aparecer como fato.
- Dois programas diferentes que receberam o mesmo número de processo dentro de uma ocorrência não
  são mais somados quando o processo mais pesado é escolhido, então o suspeito aponta o programa
  certo.
- A sequência de uma ocorrência agora cita o disco, ou o núcleo do processador, que chegou ao valor
  de pico que ela mostra. Quando dois discos passavam de um limite no mesmo intervalo, ela citava o
  disco que passou primeiro ao lado do pico do outro.
- Um programa sem resposta que ligava dois incidentes próximos agora os junta numa só ocorrência.
  Antes, os dois eram salvos como ocorrências separadas com intervalos sobrepostos, o que contava o
  mesmo momento duas vezes na bandeja.
- Uma ocorrência de um travamento longo ou de um programa sem resposta por muito tempo, cujos
  primeiros segundos já tinham saído dos dez minutos que o monitor mantém na memória, agora é
  marcada como incompleta, em vez de parecer completa.
- A página de privacidade e a página sobre como relatar um problema agora dizem que os detalhes que
  o Windows registra para um evento, que o Vestigium guarda e mostra, podem incluir o caminho de um
  programa ou arquivo e, portanto, o seu nome de usuário.
- Uma configuração de retenção fora da faixa ou ilegível não redefine mais as outras duas. Só o
  valor inválido volta ao padrão, e o monitor registra qual deles substituiu.
- Uma captura profunda cuja ferramenta de quadros escreveu linhas, mas nenhum quadro utilizável,
  agora mostra os quadros como não medidos, com o motivo, em vez de como uma execução sem quadros. O
  mesmo vale para uma saída sem um cabeçalho que o monitor consiga ler, como um nome de programa com
  vírgula.
- Um programa iniciado de uma pasta muito profunda agora mostra o caminho completo. O caminho se
  perdia quando passava de cerca de 1.000 caracteres, e o programa aparecia sem arquivo.
- Um registro de inicialização que falha durante a leitura não é mais tomado por uma inicialização
  sem nada lento. O exame agora conta como não lido, então a tela Saúde não mostra resultado de
  inicialização em vez de mostrar um resultado limpo, e o exame guarda a contagem dos eventos de
  inicialização que não conseguiu ler. Os exames salvos pela versão anterior continuam sendo lidos.
  O auxiliar e o monitor precisam ser atualizados juntos, porque o formato do arquivo do exame
  mudou.
- Um arquivo de caixa-preta danificado não pode mais deslocar o intervalo da análise pós-morte
  depois de um desligamento inesperado. Registros com hora anterior a 2020 ou mais de um dia à
  frente do relógio são ignorados, e um arquivo salvo por outra versão do monitor é informado como
  uma parada que não foi limpa, em vez de uma parada limpa.
- Depois de uma inicialização, a verificação de saúde agora roda de novo assim que o primeiro lote
  de eventos do Windows é salvo. A verificação de um minuto depois da inicialização não via os
  eventos de boot e de falha que ainda esperavam para ser salvos, então um reinício inesperado
  aparecia nos alertas de saúde com até uma hora de atraso.
- Um disco que foi removido do PC não mantém mais um alerta de saúde. A verificação de hora em hora
  guardava a última leitura de cada disco até o monitor reiniciar; agora ela esquece os discos que
  não estão mais no PC, e o alerta deles fecha na próxima verificação.
- A verificação de saúde de hora em hora não acorda mais um HD ocioso cujo tipo o Windows não
  informa, como um atrás de uma controladora RAID ou de um adaptador de host. Um disco de tipo
  desconhecido só é lido enquanto está em uso ou quando é um NVMe.
- O ícone da bandeja agora se corrige quando o Windows rejeita uma atualização dele. Se o ícone ou a
  dica não puderam ser alterados, por exemplo enquanto o Explorer reiniciava, o monitor tenta de
  novo na próxima atualização, em vez de mostrar o nível antigo até o nível mudar outra vez.
- Iniciar com o Windows agora acompanha o programa quando ele muda de lugar. Se a entrada aponta
  para outra cópia do Vestigium, o monitor a reescreve para si mesmo na próxima inicialização, a
  menos que você a tenha desativado no Gerenciador de Tarefas, e nunca cria uma entrada que não
  existia. Desinstalar o Vestigium agora remove a entrada de inicialização dele, enquanto uma
  atualização a mantém.
- Quando o PC está prestes a entrar em suspensão ou hibernar, o monitor agora salva antes o que tem
  na memória. O aviso de suspensão acorda a coleta na hora e espera até um segundo e meio para ela
  salvar o lote aberto, os eventos pendentes, as lacunas e a caixa-preta; antes, isso só era feito
  depois que o PC acordava, ou se perdia se ele nunca acordasse.
- Uma verificação de volume agora pode ser parada, e não roda mais sem limite. Parar encerra o
  chkdsk em um ou dois segundos, e a verificação é registrada como não concluída; uma verificação
  que ainda está rodando depois de seis horas é encerrada do mesmo jeito.
- Parar uma verificação de superfície agora tem efeito em segundos, mesmo num disco falhando. Antes,
  o pedido só era conferido entre blocos de 1 MiB, e um bloco que falhava era lido setor por setor
  sem limite, o que podia levar minutos. Agora a parada é conferida antes de cada setor e durante
  uma leitura em andamento, uma leitura ainda sem resposta depois de 30 segundos é cancelada, e um
  bloco que leva mais de 30 segundos para ser delimitado conta como ilegível. Um bloco interrompido
  por Parar fica fora do mapa e é lido de novo quando a verificação é retomada.
- A leitura dos dumps não trava mais num dump que o mecanismo de depuração do Windows não consegue
  carregar. Cada dump tem no máximo um minuto, a leitura inteira no máximo dez minutos, e um dump
  deixado fora da análise continua listado com tamanho e data.
- Uma captura profunda não espera mais para sempre pelo registrador de quadros. Se o PresentMon
  ainda está rodando 15 segundos depois do momento em que a captura deveria ter terminado, ele é
  encerrado e a captura termina com os quadros lidos até ali, ou é marcada como não medida quando
  não havia nenhum. O PresentMon também fica vinculado ao auxiliar de administrador, então encerrar
  o auxiliar também o encerra, e o monitor para, ao iniciar, uma sessão de rastreamento do
  PresentMon que tenha sobrado, como já fazia com a do kernel.
- As entradas de inicialização de antivírus comuns, como Avast, AVG, ESET, Kaspersky, Norton, McAfee
  e Bitdefender, agora aparecem como protegidas na tela Otimizações e não podem mais ser desativadas
  por ela. Antes, só nomes com palavras como Defender ou Security eram protegidos. Uma inicialização
  lenta também só é atribuída a uma entrada quando os nomes são do mesmo programa, não quando têm
  quatro letras em comum, então uma entrada chamada Sync não é mais culpada pela demora do Dropbox
  Sync Helper.
- A comparação de antes e depois na tela Otimizações não mostra mais resultado para uma alteração
  feita há tanto tempo que os dias anteriores a ela ficam fora dos 30 dias de histórico que o
  monitor guarda. Antes, uma alteração aplicada há mais de 23 dias era comparada com um período
  anterior incompleto ou vazio, e podia indicar menos problemas ou nenhuma diferença por engano.
- A comparação de antes e depois na tela Otimizações agora compara com que frequência o problema
  aconteceu por hora de monitoramento, e não as contagens brutas. Antes, 3 engasgos num dia em que o
  PC ficou ligado 20 horas e 2 num dia em que ficou ligado 2 horas apareciam como menos, e qualquer
  diferença era chamada de menos ou mais. As duas barras agora mostram a taxa por hora monitorada,
  desenhadas a partir do zero, com a contagem e as horas por trás de cada uma, como 18 em 168 h. Ela
  só diz menos ou mais com pelo menos 5 eventos nos dois períodos juntos, pelo menos 12 horas
  monitoradas em cada período e uma taxa pelo menos o dobro da outra; caso contrário, diz que ainda
  não há dados suficientes, desenha contornos tracejados e diz o que falta. O texto não chama mais
  uma diferença pequena de significativa.
- Os dumps guardados numa subpasta da pasta de dumps não somem mais da lista e da contagem de espaço
  até a próxima leitura dos dumps. A listagem do próprio monitor agora lê um nível de subpastas,
  como a leitura dos dumps faz, e não marca mais esses dumps como apagados.
- Ler ou apagar dumps com a senha de administrador de outra conta agora lê os dumps da conta que
  pediu, e não os do perfil do próprio administrador. A janela informa a conta ao auxiliar, que
  encontra o perfil dessa conta no Windows e recusa tudo que não seja uma pasta local. O auxiliar
  também apaga os dumps que lê numa subpasta da pasta Minidump, o que antes recusava. A janela e o
  auxiliar são distribuídos juntos.
- Apagar dumps agora diz por que alguns não foram apagados. Depois de uma exclusão parcial, a
  mensagem acrescenta quantos foram recusados (não permitidos, ou que não existiam mais) e quantos
  não puderam ser apagados, em inglês e em português; antes, ela só dizia que os outros não existiam
  mais ou estavam fora das pastas de dumps. Apagar um único dump na página dele, quando o auxiliar
  recusa, agora mostra a mesma mensagem em vez de informar 0 B liberados. A janela e o monitor
  precisam ser atualizados juntos, porque o protocolo de comunicação entre eles mudou.
- Um alerta de saúde sobre dois antivírus ativos, ou sobre o arquivo de paginação, agora continua
  aberto quando o Windows não responde à verificação. Antes, o alerta fechava, ou nunca abria,
  sempre que a Central de Segurança do Windows ainda não estava disponível (muitas vezes logo depois
  do logon) ou o arquivo de paginação não podia ser lido, e uma verificação de antivírus com falha
  só era repetida depois de um dia. Agora a verificação é repetida na próxima rodada de verificação
  de saúde.
- Um programa que continua sem resposta 15 segundos ou mais depois de o PC sair da suspensão ou de o
  monitor retomar de uma pausa agora ganha um relatório. Antes, um travamento visto pela primeira
  vez logo depois de uma retomada era sempre descartado como o tempo de acomodação do despertar,
  então um programa que ficava congelado por minutos nunca aparecia.
- Um relatório com um número muito grande de entradas, como um gerado por milhares de eventos
  repetidos do Windows, não é mais salvo no histórico num formato que não podia ser aberto de novo:
  o histórico agora lê relatórios até quatro vezes maiores que antes, então um relatório que foi
  guardado, mas mostrava um erro, abre.
- Uma enxurrada de eventos repetidos do Windows, como um disco ou uma controladora de armazenamento
  registrando o mesmo erro muitas vezes por segundo, não enche mais um relatório com milhares de
  entradas nem se divide num relatório a cada poucos minutos. Um relatório agora guarda os 16
  primeiros eventos de cada provedor e número de evento, e os eventos de um mesmo provedor com até
  dois minutos de distância entre si se juntam num só relatório. Um relatório com um evento assim
  agora aparece cerca de dois minutos depois do evento, em vez de meio minuto, porque espera por
  mais.
- Um relatório que deixou de fora repetições do mesmo evento do Windows agora diz quantas foram,
  numa linha como "Mais 1.200 repetições do mesmo evento não foram guardadas", em inglês e em
  português. A janela e o monitor precisam ser atualizados juntos, porque o relatório guardado e o
  protocolo de comunicação entre eles mudaram.
- Uma enxurrada de eventos repetidos do Windows não tira mais os outros eventos da sequência de um
  relatório nem faz o histórico crescer sem limite. A sequência de um relatório guarda no máximo 16
  eventos de cada provedor e número de evento a cada dois minutos, e o histórico guarda no máximo 64
  de cada por lote de cinco minutos, o que ainda fica muito acima das contagens que as verificações
  de saúde procuram.
- Um programa que ainda estava sem resposta quando o monitoramento parou, foi pausado ou o PC entrou
  em suspensão agora ganha um relatório que termina nesse momento, com a linha "o programa ainda não
  respondia quando o monitoramento parou". Antes, um travamento em aberto nesse momento era
  esquecido e nunca aparecia. A janela e o monitor precisam ser atualizados juntos, porque o
  relatório guardado e o protocolo de comunicação entre eles mudaram.
- O relatório de um travamento não afirma mais como fato que o PC inteiro parou. A linha agora diz
  "a sonda do monitor ficou parada por 6,8 s; o PC provavelmente travou", porque um atraso do
  próprio processo do Vestigium, como o monitor ser suspenso, parece para ele igual a uma parada do
  PC. A linha tem esse texto em inglês e em português.
- Um programa que parou de responder agora só é apontado com confiança alta quando o uso de
  processador ou de disco dele aparece durante o travamento, ou quando nada mais no sistema explica
  a parada. Antes, qualquer janela sem resposta colocava o programa dela em primeiro lugar, à frente
  de uma causa de memória ou de disco que foi medida, então um programa que só esperava por um PC
  sem recursos era apontado como culpado.
- Uma falta de memória agora aponta o programa que mais ocupava memória nos 30 segundos antes do
  travamento, e não o maior do relatório inteiro. Antes, podia ser apontado um programa que cresceu
  depois que o travamento começou, ou um que tinha fechado e sido substituído por outro maior, e a
  recomendação era fechar o programa errado.
- O título em inglês de um disco cujo SMART prevê falha não coloca mais dois-pontos depois do nome
  da unidade, o que mostrava dois-pontos duplicados para uma letra de unidade. Agora ele diz "SMART
  of disk C: predicts failure".
- Hoje, Todos os eventos e a página de um programa não deixam mais de abrir quando dois períodos de
  sobrecarga começam no mesmo instante, como dois discos em atividade máxima juntos ou o processador
  e a memória enchendo ao mesmo tempo. Antes, a lista Eventos do dia e a tabela de Todos os eventos
  mostravam um erro pelo resto do dia.
- Uma parada não é mais informada como desligamento inesperado do PC quando o Windows não consegue
  dizer quando o PC caiu. Sem o registro de desligamento inesperado que data a queda, o relatório
  agora só é criado quando o evento de energia veio até cinco minutos depois do último segundo que o
  monitor viu; se o monitor foi encerrado à força e o PC caiu muito depois, o tempo fora fica como
  uma parada simples, em vez de um desligamento inesperado datado no último segundo visto.
- O relatório de um PC que desligou de forma inesperada agora inclui os eventos de disco e de vídeo
  que o Windows registrou nos dois minutos antes do último segundo que o monitor viu. Antes, eles
  chegavam depois que o relatório era criado, então um travamento causado pelo disco ou pelo driver
  de vídeo era informado como uma queda de energia de confiança baixa; agora ele aponta o
  armazenamento ou o driver de vídeo.
- Configurações, apagar o histórico, importar resultados e aplicar um ajuste não ficam mais
  indisponíveis pelo resto da execução depois que o histórico não pôde ser aberto uma vez ao
  iniciar: o monitor tenta de novo no próximo pedido que precisa dele e importa os resultados que
  estavam esperando. Um local de histórico que não aceita o modo WAL, como uma pasta de rede, agora
  é recusado com um erro que cita o modo encontrado, em vez de ser usado sem segurança.
- Um arquivo de histórico com cabeçalho danificado não impede mais o monitor de iniciar a cada
  logon: o monitor move o arquivo, com os arquivos -wal e -shm, para um nome que termina em .corrupt
  e a hora, nunca o apaga, e inicia um novo histórico que assume os ajustes ainda aplicados, as
  configurações e os dados do PC quando o arquivo antigo ainda pode ser lido, para que esses ajustes
  ainda possam ser desfeitos. Um arquivo que só está bloqueado, ou que uma versão mais nova salvou,
  não é tocado.
- Uma escrita no histórico que falha, por exemplo com o disco cheio ou um arquivo bloqueado, não
  perde mais os dados daquele momento: o lote é mantido e salvo junto com o próximo, até três lotes,
  e os eventos do Windows que ele continha continuam sendo lidos depois dele, então nenhum é pulado.
  Um desligamento inesperado, um travamento ou um episódio de sobrecarga que não pôde ser guardado
  também é tentado de novo, e a bandeja só o conta depois que é salvo.
- Quando a escrita no histórico continua falhando e há mais de três lotes esperando, a linha do
  tempo agora marca o trecho cujas leituras foram descartadas como "Dados não gravados.", em vez de
  deixá-lo parecendo um trecho em que nada foi monitorado. A versão do histórico passa para 13: a
  janela e o monitor precisam ser atualizados juntos, e uma janela mais antiga recusa um histórico
  que tenha a nova marca.
- Um monitor que para de responder não deixa mais a janela esperando para sempre: um pedido a ele
  agora desiste depois de três segundos. Um programa que se conecta ao canal do monitor e não diz
  nada, ou envia meio pedido, é desconectado depois de alguns segundos, em vez de bloquear a janela.
- Apagar o histórico, importar resultados, salvar configurações ou aplicar um ajuste que leva vários
  segundos não faz mais a janela informar que o monitor não responde: esses pedidos agora esperam
  até um minuto pela resposta, enquanto um pedido que só lê a memória do monitor continua desistindo
  depois de três segundos.
- Um arquivo de resultado de captura profunda, de exame de saúde ou de ajuste que o monitor não
  conseguiu apagar depois de guardar não é mais guardado de novo na próxima importação: cada um é
  mantido uma vez só. As cópias que um banco de dados já tem são removidas quando ele é aberto por
  esta versão, então a janela e o monitor precisam ser atualizados juntos.
- Um ajuste que o auxiliar de administrador começou, mas não terminou, porque foi parado ou o PC
  perdeu energia no meio, agora deixa uma entrada no histórico de ajustes em vez de nada. O auxiliar
  anota o que vai alterar antes de alterar qualquer coisa e não altera nada quando não consegue
  anotar. Cerca de dez minutos depois, o monitor compara cada configuração com o que o Windows tem:
  uma configuração com o novo valor é registrada como alterada e pode ser desfeita, uma com o valor
  antigo como inalterada, e qualquer outra como desconhecida, sem Desfazer. A janela e o monitor
  precisam ser atualizados juntos.
- A tela Otimizações agora diz o que um auxiliar interrompido deixou. No cartão do ajuste, uma nota
  em "O que muda" diz que o auxiliar parou antes de terminar e o que foi encontrado: o Windows tem o
  novo valor e Desfazer é oferecido, nada foi alterado, ou não se sabe e Desfazer não é oferecido.
  Quando só algumas configurações têm o novo valor, a nota lista cada uma com o que foi encontrado,
  e Desfazer mexe só nelas. Desfazer pede confirmação dentro da nota, dizendo exatamente o que
  volta, em vez de numa caixa de diálogo. Uma alteração que não pôde ser conferida não aparece mais
  como "Nada foi alterado": a linha dela no histórico diz "Interrompido" com o resultado, e o cartão
  mostra o valor atual como não confirmado e oferece Aplicar novamente.
- Se o canal de conexão do monitor com a janela falhar, o monitor agora registra o motivo, abre o
  canal de novo depois de um segundo e só fecha quando isso continua falhando, em vez de seguir sem
  canal enquanto a janela diz que ele não está em execução.
- Abrir o Vestigium uma segunda vez não inicia mais um segundo monitor, mesmo quando o primeiro
  perdeu o canal de conexão com a janela, então dois monitores nunca salvam no mesmo histórico.
- Quando outro programa já está usando o canal de conexão do Vestigium, o Vestigium agora avisa numa
  mensagem, nomeando o programa quando o Windows permite, em vez de fechar sem dizer nada.
- Depois que o monitor reinicia, a janela agora se reconecta sozinha, em vez de mostrar por alguns
  segundos o aviso sobre outro programa ou outra versão e depois o botão Iniciar o Vestigium. Um
  pedido que talvez já tenha sido executado, como apagar o histórico, nunca é enviado uma segunda
  vez.
- As telas de histórico, inclusive Hoje, agora abrem direto do banco de dados enquanto o monitor
  está travado ou ocupado, em vez de congelar até ele responder; até ele responder, só faltam os
  últimos minutos ainda não salvos. Aplicar ou desfazer um ajuste não congela mais as outras telas
  enquanto o pedido de permissão está aberto ou o auxiliar elevado roda.
- Uma verificação de superfície agora anota quando outros programas usavam o disco enquanto ela
  rodava, mostra isso no resultado com a sugestão de repeti-la com o PC ocioso, e não abre mais por
  isso o alerta de atenção "Leitura muito lenta em regiões" num HD. Setores ilegíveis continuam
  críticos. Os resultados da verificação agora são guardados num novo formato, então a janela, o
  monitor e o auxiliar precisam ser atualizados juntos; os resultados anteriores continuam sendo
  lidos.
- Uma segunda captura profunda iniciada enquanto outra já está em andamento, por exemplo depois de a
  janela ser fechada e reaberta, agora é recusada pelo auxiliar de administrador, em vez de
  interromper o registro da primeira e deixá-la truncada. O auxiliar também não aceita mais um
  caminho de arquivo de rastreamento.
- Uma captura profunda agora atribui cada rotina lenta de driver ao driver que estava carregado
  naquele endereço quando ela rodou, então um driver que ocupou o lugar de outro já descarregado não
  é mais culpado pelas rotinas do anterior. Um quadro travado também é associado ao atraso que a
  sonda viu ao longo do tempo que cada um durou, então atrasos de mais de 50 ms confirmam um ao
  outro, e os horários da captura acompanham o relógio do PC numa captura longa.
- A pesquisa na lista Programas agora procura em todos os programas que o monitor informa, e não só
  nos 100 com maior uso de processador, então um programa mais abaixo na lista é encontrado pelo
  nome.
- Desfazer não sobrescreve mais uma configuração que você mesmo mudou depois de aplicar um ajuste.
  Se o Windows agora tem um valor diferente do que o Vestigium definiu, o desfazer é recusado com
  uma mensagem e nada é alterado, e um desfazer recusado não marca mais a alteração original como
  desfeita. O auxiliar de administrador também recusa um plano que pede um valor fora do que a
  configuração aceita, e cada valor alterado é lido de volta para conferência.
- Um ajuste que altera um serviço do Windows, ou uma configuração de energia, e depois falha numa
  etapa seguinte não deixa mais a primeira alteração para trás sem volta. O tipo de inicialização ou
  o valor de energia antigo agora é restaurado, e o histórico mostra o erro real. Se nem isso puder
  ser restaurado, a alteração fica como alterada em parte, com Desfazer oferecido, em vez de
  aparecer como recusada.
- Desfazer um ajuste de energia, como a Economia de energia do PCI Express ou a Suspensão seletiva
  USB, agora devolve o valor antigo ao plano de energia que foi alterado, e não ao plano que estiver
  ativo quando você clica em Desfazer, e recusa com um erro se esse plano não existe mais. Antes,
  trocar de plano entre aplicar e desfazer deixava o primeiro plano alterado e mudava o segundo. As
  alterações feitas antes desta atualização são desfeitas como antes. O monitor e a janela mudaram a
  forma de se comunicar, então precisam ser atualizados juntos.
- Um auxiliar que para no meio, por disco cheio, queda de energia ou porque o processo foi
  encerrado, não deixa mais um arquivo de resultado vazio que o monitor lê de novo a cada
  importação. O auxiliar agora salva o resultado com um nome temporário e só dá o nome final quando
  ele está completo, e o monitor apaga um resultado vazio ou um arquivo temporário que não mudou por
  uma semana.
- Um ajuste não é mais aplicado ao Windows quando o ponto de restauração que ele pediu não foi
  confirmado, a menos que você tenha aceitado continuar sem ponto. Antes, um ponto que o Windows
  registrou e depois descartou, ou que o Windows nunca confirmou, deixava a alteração passar
  enquanto o histórico dizia que um ponto tinha sido criado.
- A janela não diz mais que a Restauração do Sistema está desativada quando o Windows não conseguiu
  criar o ponto de restauração. Quando um ajuste é retido por falta de ponto, o diálogo agora tem um
  título próprio para Restauração do Sistema desativada, para um ponto que falhou e para um ponto
  que o Windows não confirmou, e mostra o trajeto do pedido: Pedido ao Windows, O Windows criou o
  ponto, Ponto na lista. A etapa em que parou fica marcada, com o número do erro do Windows quando
  houver. Tentar de novo pede o ponto mais uma vez com o diálogo ainda aberto e avisa quando termina
  do mesmo jeito; Continuar sem ponto é um botão separado. Um ponto que o Windows registrou como
  criado, mas tirou da lista, agora aparece como não confirmado no histórico e no diálogo.
- Uma verificação de superfície num disco que desaparece no meio, como quando um cabo USB é puxado
  ou a porta entra em suspensão, agora termina com falha porque o disco parou de responder, e uma
  verificação posterior continua de onde parou. Antes, todos os blocos depois desse ponto eram
  registrados como ilegíveis e a recomendação era trocar o disco. Uma área realmente danificada de
  um disco que ainda responde continua sendo informada como ilegível.
- Um HD Seagate que registrou um único tempo esgotado de comando não mostra mais uma contagem
  impossível de 4294967295 setores danificados. O atributo de tempo esgotado de comando agora é lido
  como o contador de 16 bits que ele contém, e o número num alerta de saúde do disco é o maior dos
  contadores monitorados, e não a soma deles.
- Um HD que estava ocioso quando o monitor reiniciou não tem mais o alerta de saúde do disco em
  aberto fechado e reaberto como um novo, o que podia esconder um alerta crítico e zerar a data
  dele. O alerta agora fica aberto até o disco ser lido de novo, e o disco ocioso continua nunca
  sendo acordado para a leitura.
- Um relógio do PC que volta no tempo, como acontece numa correção de hora depois que um PC com dois
  sistemas operacionais inicia, não esconde mais travamentos, programas sem resposta e engasgos pelo
  tempo que voltou. Antes, um trecho de monitoramento perdido registrado enquanto o relógio estava
  adiantado continuava cobrindo o tempo seguinte e descartava em silêncio tudo de novo que tocasse
  nele.
- Um relógio do PC adiantado demais, por uma correção de hora, uma mudança manual ou um segundo
  sistema operacional no mesmo PC, não faz mais o monitor apagar histórico real: a limpeza de hora
  em hora agora pula uma rodada quando o relógio está mais de uma hora à frente dos dados guardados
  mais recentes.
- Um relógio do PC que salta, por uma correção de hora, uma mudança manual ou um segundo sistema
  operacional, não faz mais um programa sem resposta ou um engasgo durar o salto inteiro, não
  inventa um engasgo nem enche a evidência de um travamento com momentos de outra hora. O monitor
  agora percebe o salto, fecha o que estava aberto, salva, recomeça a memória curta e registra a
  mudança na linha do tempo como "Relógio do PC alterado".
- Um relógio do PC que volta no tempo não impede mais o monitor de salvar o histórico pelo tempo que
  voltou, o que podia perder minutos de dados se o PC desligasse de forma inesperada nesse
  meio-tempo. Um lote que ficou muito grande agora é salvo em várias partes, para que o histórico
  sempre consiga lê-lo de volta; antes, um recuo de cerca de um dia podia deixar um lote grande
  demais para ser lido.
- A memória do monitor não cresce mais cerca de 3,5 MB a cada hora que o relógio do PC volta. Os
  rankings de programas dos últimos dez minutos agora são limitados pela quantidade, além da idade,
  e a lista de nomes de programas deles só é refeita quando dobra de tamanho, o que também economiza
  processador num PC com muitos programas de vida curta.
- O gráfico de dez minutos da tela Agora não para mais de crescer depois que o relógio do PC volta
  no tempo. Antes, cada ponto novo parecia mais antigo que o último e era ignorado até o relógio
  alcançar; agora os pontos que ficam à frente da nova hora são descartados e o gráfico continua
  dali.
- Um relógio do PC que salta não alonga nem encurta mais o modo sensível, uma pausa programada ou a
  espera pelo registro de um desligamento inesperado no Windows. Esses tempos agora usam um relógio
  estável, que continua contando enquanto o PC está em suspensão e ignora mudanças de hora, então
  uma pausa de trinta minutos continua terminando trinta minutos depois e a hora de término que a
  janela mostra acompanha o relógio corrigido. Um desligamento inesperado cujo último segundo
  monitorado fica à frente de um relógio que foi atrasado continua sendo reconhecido.
- O monitor não pode mais travar de vez quando a janela pede o status dele exatamente no momento em
  que o ícone da bandeja se atualiza. Antes, duas threads dele podiam ficar uma esperando a outra, o
  que parava o menu da bandeja e o ícone, impedia que os avisos de suspensão fossem tratados e
  deixava a janela mostrando um estado desatualizado até o Windows encerrar o monitor.
- Uma instalação, atualização ou desinstalação que encontra o monitor ainda rodando depois de pedir
  que ele feche e de encerrá-lo agora para com uma mensagem, antes de alterar qualquer arquivo, em
  vez de seguir em frente. A mensagem diz para escolher Sair no ícone da bandeja e rodar o
  instalador de novo.
- Uma captura profunda interrompida porque o auxiliar foi encerrado à força não deixa mais a sessão
  de rastreamento do kernel rodando: o monitor a para quando inicia, desde que nenhum auxiliar
  esteja rodando.
- Uma captura profunda não dá mais confiança alta a um driver, nem manda atualizá-lo, por causa de
  uma única rotina longa. Um driver agora precisa de três rotinas de 1 ms ou mais, ou de uma perto
  de um atraso ou de um quadro perdido; uma rotina isolada é informada com confiança média, na
  captura e na ocorrência que ela cobre.
- Consultar o que uma captura profunda viu em torno de uma ocorrência não deixa mais escapar uma
  rotina longa ou um quadro perdido só porque uma captura longa teve piores em outro momento: a
  captura agora guarda os piores de cada minuto, e o monitor guarda os atrasos da sonda por um dia
  inteiro em vez de 15 minutos, então uma captura aberta mais tarde ainda os mostra. O formato da
  captura muda, então a janela, o monitor e o auxiliar precisam ser atualizados juntos; as capturas
  já guardadas continuam abrindo.
- Uma captura profunda de qualquer duração agora fica dentro de 8 MiB depois de guardada: se os
  piores de cada minuto ocupassem mais, ela descarta primeiro as rotinas, os quadros perdidos e os
  atrasos mais curtos, então uma captura movimentada de um dia inteiro não pode fazer o histórico
  crescer sem limite.
- Uma captura profunda cujo registrador de quadros parou no meio não aparece mais como se os quadros
  tivessem sido medidos na captura inteira: a janela agora diz que os quadros não foram medidos e
  por quê. O formato da captura muda, então a janela, o monitor e o auxiliar precisam ser
  atualizados juntos.
- Uma captura profunda cujo rastreamento de drivers não pôde ser lido, ou não recebeu nenhum evento,
  agora diz que as medições de drivers não estão disponíveis, em vez de informar que zero drivers
  foram medidos.
- Um programa com mais de 5 % de quadros lentos numa captura profunda não some mais da página da
  captura. Ele fica na lista de aplicativos com a sua parcela de quadros lentos e uma marca "Acima
  de 5 %", e o nível Técnico acrescenta as duas coisas à tabela dele. Ele continua nunca sendo
  apontado como suspeito, porque um aplicativo que só desenha sob demanda parece lento quando está
  parado.
- A página de uma captura profunda agora diz o que cada fonte mediu e o que guardou. Um novo bloco,
  "O que foi medido", lista os drivers, os quadros e a sonda de atrasos com uma barra de zero até a
  duração da captura, "3 de 5 min" quando o registrador de quadros parou no meio ou "Não medido"
  quando uma fonte falhou, e o que a captura guardou de cada um: as três piores chamadas longas de
  driver e os três piores quadros lentos por minuto, e todos os atrasos. O cabeçalho ganha uma marca
  Cobertura ao lado de Resultado, as linhas do histórico de capturas também a mostram, e uma linha
  diz até onde uma fonte chegou, como "quadros 3 de 5 min". As notas antigas sobre quadros e drivers
  não medidos foram substituídas por esse bloco, e o que fazer a respeito passou para os passos da
  página. Quadros medidos só em parte da captura não escondem mais a lista de aplicativos.
- Uma captura profunda que não conseguiu medir tudo não diz mais que não houve engasgos. Sem culpado
  encontrado, o título agora diz "Não dá para dizer se houve engasgos" quando os drivers ou os
  quadros não foram medidos, ou só em parte, e oferece Capturar por 10 minutos; "Nenhum culpado
  identificado" quando os atrasos foram medidos ou um aplicativo teve muitos quadros lentos; e
  mantém "Nenhum engasgo durante a captura" só para uma captura que mediu tudo e não viu nada. Uma
  captura que aponta um culpado continua apontando, com a marca Cobertura ao lado. A marca Resultado
  tem um estado para cada caso, entre eles "Indeterminado", "Sem culpado" e "Aplicativo suspeito",
  as mesmas palavras encabeçam as linhas do histórico, e o que fazer sobre uma fonte que ficou
  incompleta aparece nos passos.
- Uma execução do Diagnóstico de Memória do Windows não quebra mais Histórico, Todos os eventos,
  Período de sobrecarga, Dumps e Configurações com uma página de erro, e os erros de memória que ela
  encontra agora abrem um alerta na Saúde. O histórico já registrado dessa forma é corrigido quando
  o monitor inicia. O formato do histórico muda, então a janela e o monitor precisam ser atualizados
  juntos.
- Uma verificação de superfície ainda em andamento agora mostra o progresso quando você fecha e
  reabre a janela, mantém os botões de teste ocupados e pode ser parada pela janela.
- O teste de velocidade do disco não inicia mais num volume com menos de 1,5 GiB livres, que é o
  arquivo de 1 GiB dele mais uma margem; um volume tão pequeno ficava completamente cheio enquanto o
  teste rodava. Em vez disso, a tela Testes agora diz quanto espaço é necessário.
- O teste de velocidade do disco agora apaga o arquivo temporário de 1 GiB que uma execução anterior
  deixou para trás (depois de uma queda de energia, por exemplo), e quando o disco para de responder
  ele espera as leituras e escritas canceladas terminarem antes de liberar a memória que elas usam.
- A verificação de volume agora registra se o Windows tinha o volume marcado para reparo antes e
  depois da execução, o que mostra quando a própria verificação o marcou. O formato do resultado do
  teste muda, então a janela, o monitor e o auxiliar precisam ser atualizados juntos; os resultados
  salvos antes continuam sendo lidos.
- O monitor não para mais de medir pela sessão inteira quando os contadores de desempenho do Windows
  estão ausentes ou danificados ao iniciar, ou quando o temporizador de precisão dele não pode ser
  criado. Ele continua registrando travamentos, desligamentos inesperados e eventos, tenta abrir os
  contadores de novo com uma espera crescente de até um minuto e retoma as leituras por segundo
  assim que eles abrem. Antes, nada era registrado até o monitor ser reiniciado, e o desligamento
  inesperado da sessão anterior se perdia.
- Uma leitura que falha no monitor (memória, tempos do processador, os contadores de desempenho ou a
  lista de processos) não cancela mais o resto daquele segundo. As outras leituras, a detecção de
  travamentos e de desligamentos inesperados, os eventos pendentes e as escritas periódicas
  continuam, e a lista salva dos discos do PC não é mais esvaziada quando os contadores de disco não
  devolvem nada. Antes, uma leitura que continuava falhando deixava inacabados os travamentos que o
  monitor já tinha visto e os eventos sem salvar.
- Um contador de desempenho que o Windows informa como inválido num segundo, por exemplo uma taxa
  ainda sem dados, não faz mais perder o segundo inteiro nem aparece como zero. Só esse valor fica
  de fora: o processador, a memória e os outros contadores continuam sendo registrados, e as regras
  de limitação da CPU e de disco, os resumos por minuto e a caixa-preta pulam um valor que não foi
  lido. Um disco cujo contador de latência, fila ou taxa de transferência é inválido não mostra mais
  zero nele. O histórico guarda cada segundo num novo formato, então a janela, o monitor e o
  auxiliar precisam ser atualizados juntos; o histórico salvo antes continua sendo lido.
- Um segundo cujos contadores de desempenho não podem ser lidos de jeito nenhum, porque estão
  ausentes, danificados ou falhando, agora é registrado mesmo assim, com as leituras de processador
  e de memória e sem os valores dos contadores. Antes, um segundo assim ficava sem registro, e o dia
  aparecia sem dados. Um contador de disco que ganha instâncias enquanto é lido não perde mais as
  leituras de disco daquele segundo.
- Um evento registrado logo antes de um travamento não desvia mais a atenção do relatório para longe
  do travamento: quando uma ocorrência tem um travamento e um evento anterior, as causas são
  avaliadas em torno do travamento, então um processador ocupado ou um disco lento perto do
  travamento é encontrado, em vez de a causa aparecer como incerta.
- Um evento de disco ou um reinício do driver de vídeo no registro de eventos não vira mais o
  primeiro suspeito só porque caiu a menos de um minuto e meio de um travamento. Só contam os
  eventos de 10 segundos antes do travamento até 5 segundos depois dele; um mais distante não
  esconde mais um processador ocupado ou outra causa medida. Um evento de disco perto do travamento
  só é informado com confiança alta quando o disco também respondia devagar (mais de 200 ms) nesse
  intervalo, e com confiança média nos outros casos.
- Os próximos passos para uma causa de disco agora seguem o tipo de evento que o Windows registrou,
  e não uma lista fixa. Um bloco defeituoso ou um erro de paginação pede para fazer backup dos dados
  e testar o disco; uma operação de disco repetida pede para testar o disco primeiro, depois
  verificar o cabo e atualizar o driver; os passos de driver e de cabo só vêm primeiro para um
  reinício da conexão com o disco.
- A frase do relatório para uma causa de disco agora diz o que o Windows registrou. Um reinício da
  conexão com o disco mantém o texto; uma operação de disco repetida e um bloco defeituoso ou erro
  de paginação ganham cada um a sua própria frase, e quando as leituras do disco estavam normais a
  frase diz isso e que o disco pode não ser a causa, em inglês e em português. O passo de fazer
  backup dos arquivos agora cita o disco, em vez de mostrar um marcador de posição vazio.
- Um lote de histórico que cobre mais de uma hora do relógio, por exemplo depois que o relógio do PC
  saltou para a frente e voltou, agora é guardado em várias partes de no máximo uma hora cada, para
  que a leitura do histórico sempre o encontre.
- A tela Início e o painel lateral não dizem mais "Nenhum travamento hoje", "Nenhum recomendado" ou
  "Nenhum alerta" com uma marca verde quando o que resumem não pôde ser lido. Quando os travamentos
  de hoje, os ajustes ou as verificações de saúde não carregam antes de terem sido lidos alguma vez,
  o cartão diz "Sem leitura das marcas de hoje", "Sem leitura dos ajustes" ou "Sem leitura das
  verificações de saúde" com uma marca neutra, e o painel lateral deixa as contagens de fora;
  enquanto a primeira leitura ainda está em andamento, o cartão mostra Carregando e nenhuma marca.
  Uma atualização posterior que falha mantém as contagens já lidas.
- A tela Saúde não diz mais "Tudo normal" nem marca uma verificação como boa quando ela não foi
  lida. Uma verificação que a última rodada de verificação de saúde não conseguiu ler, como a
  consulta de antivírus quando o Windows não responde, ou um disco que não respondeu à consulta de
  saúde, agora mostra "Não verificado" com uma marca neutra. Quando a verificação de saúde de hora
  em hora não termina há três horas, cada verificação que ela cobre mostra "Não verificado desde" a
  hora da última que terminou, a frase de abertura diz isso em vez de "Tudo normal", e a linha
  abaixo do título mostra a última verificação concluída e a tentativa que falhou. Um HD deixado de
  lado por estar ocioso não conta como não lido. Quando o monitor não responde, a tela diz que a
  lista é só a última salva. A tela Memória, o cartão de saúde de Hoje e o painel lateral seguem as
  mesmas marcas, e um monitor parado não acrescenta nada.

### Security

- A janela agora se recusa a iniciar o auxiliar de administrador quando a conta poderia
  substituí-lo, ou renomear ou reescrever uma pasta no caminho até ele, então um programa rodando
  numa conta padrão não consegue mais trocar o auxiliar antes que a senha de administrador de outra
  conta seja digitada no pedido de permissão. O auxiliar instalado fica em Program Files, que só um
  administrador pode alterar.
- A compilação das versões não tem mais a chave de assinatura das atualizações no ambiente. O
  instalador é compilado sem nenhum segredo, uma tarefa separada que recebe só o instalador o
  assina, e uma terceira tarefa tem as permissões para publicar o rascunho da versão, então uma
  dependência de compilação comprometida não pode usar a chave nem alterar o instalador antes de ele
  ser assinado. As GitHub Actions que a publicação usa estão fixadas em commits específicos.
- Um plano de ajustes não pode mais ser trocado enquanto o pedido de permissão de administrador está
  aberto. A janela agora mantém o arquivo do plano bloqueado contra escrita e passa ao auxiliar o
  SHA-256 dele, e o auxiliar se recusa a executar um plano que não corresponda a ele.
- O auxiliar de administrador agora carrega as bibliotecas do Windows de que precisa só da pasta de
  sistema do Windows, então um arquivo com o nome de uma delas deixado ao lado do auxiliar nunca é
  executado com direitos de administrador.
- A leitura dos dumps não segue mais uma junção ou um link dentro de uma pasta de dumps, então um
  link plantado ali não pode fazer o auxiliar de administrador ou o monitor listar ou ler outro
  dump.
- O auxiliar de administrador agora confere, pelo próprio arquivo de resultado aberto, que ele foi
  criado na pasta pedida, e o apaga pelo mesmo identificador se um link foi colocado no caminho,
  então um arquivo de resultado não pode mais acabar em outra pasta.
- O auxiliar de administrador agora lê um plano de ajustes ou uma lista de dumps a apagar por um
  único identificador aberto em que ninguém mais pode escrever, e remove o arquivo de progresso da
  verificação de superfície pelo identificador que o criou, então uma pasta trocada por um link
  enquanto uma tarefa roda não pode desviar essas etapas.
- Os ajustes feitos com direitos de administrador agora também são registrados num diário que só
  administradores podem alterar, uma pasta em ProgramData que o auxiliar cria e confere antes de
  cada uso. O histórico de um ajuste fica na sua própria pasta de dados, em que qualquer programa
  rodando como você pode escrever, então uma entrada forjada podia mostrar um Desfazer que fazia o
  auxiliar aplicar um valor que ninguém tinha escolhido. Agora o monitor só mantém uma alteração no
  histórico quando o diário a descreve, a janela recusa um Desfazer que o diário não tem antes de
  pedir permissão, e o auxiliar só executa o Desfazer de uma alteração que ele registrou, com os
  valores que registrou. Um ajuste feito antes desta versão não tem entrada no diário e ainda pode
  ser desfeito uma vez, com as conferências que todo desfazer já tem: o valor precisa ser um que a
  configuração aceita e o Windows ainda precisa ter o que foi aplicado. A janela, o monitor e o
  auxiliar precisam ser atualizados juntos.

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
