# Documentação de Divergências: Tolerância R$ 0,05 vs Saldos

## Contexto
A regra de tolerância de R$ 0,05 permite que a soma dos itens diverja do total do cupom em até 5 centavos.
Esta diferença não é distribuída nas responsabilidades individuais nem nos saldos, criando um desequilíbrio.

## Cenários de Exemplo
- Cupom total: R$ 100,00
- Soma dos itens: R$ 100,05 (diferença +R$ 0,05 dentro da tolerância)
- Responsabilidades calculadas sobre R$ 100,05 (soma dos itens)
- Saldo do pagador = R$ 100,00 (valor pago) - responsabilidade (baseada em R$ 100,05)
- Resultado: pagador "paga" R$ 0,05 a mais do que devia

## Opção 1: Ignorar a Diferença (Status Quo)
**Descrição**: Manter o comportamento atual. A diferença de até R$ 0,05 é aceita na validação mas não distribuída.
**Vantagens**:
- Simples, sem mudança de lógica
- Compatível com implementação atual
**Desvantagens**:
- Saldos não batem exatamente com valor pago
- Pequeno desequilíbrio sistemático (sempre contra quem pagou)

## Opção 2: Ajustar Responsabilidade do Pagador
**Descrição**: Subtrair a diferença da responsabilidade de quem pagou.
**Exemplo**: Diferença = +R$ 0,05 (itens > total). Pagador paga R$ 100,00. Responsabilidade do pagador reduzida em R$ 0,05.
**Vantagens**:
- Saldos fecham exatamente (pagador paga exatamente o que deve)
- Quem pagou não é prejudicado pela diferença
**Desvantagens**:
- Muda regra de divisão da casa (50/50 + centavo)
- Complexidade: diferença pode ser positiva ou negativa

## Opção 3: Distribuir Diferença Proporcionalmente
**Descrição**: Dividir a diferença entre Igor e Renan proporcionalmente às responsabilidades.
**Exemplo**: Diferença R$ 0,05. Igor responde por 60% do total → R$ 0,03. Renan 40% → R$ 0,02.
**Vantagens**:
- Justo entre as partes
- Saldos fecham
**Desvantagens**:
- Mais complexo de implementar e explicar
- Quebra a regra simples de "centavo para quem não pagou"

## Decisão
**Não implementar nesta fase (D10)**. Documentar as 3 opções e deixar para decisão futura quando houver caso real que exija resolução.
A tolerância de R$ 0,05 existe para lidar com arredondamentos de centavos em cupons fiscais reais; o impacto financeiro é mínimo.