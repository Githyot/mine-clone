;; Plugin de exemplo com tela e item novo (Lote 9). Copie para a pasta plugins/.
;; /painel abre uma janela com botoes; o item "pedra_da_sorte" (desenhado aqui mesmo, 16x16)
;; abre a mesma janela com clique direito. Quem entra no seu mundo recebe este plugin.
(module
  (import "jogo" "say" (func $say (param i32 i32)))
  (import "jogo" "set_time" (func $set_time (param f32)))
  (import "jogo" "give" (func $give (param i32 i32 i32) (result i32)))
  (import "jogo" "add_command" (func $add_command (param i32 i32)))
  (import "jogo" "add_item" (func $add_item (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
  (import "jogo" "screen_open" (func $screen_open (param i32 i32)))
  (import "jogo" "screen_text" (func $screen_text (param i32 i32)))
  (import "jogo" "screen_button" (func $screen_button (param i32 i32 i32)))
  (import "jogo" "screen_close" (func $screen_close))
  (memory (export "memory") 1)
  (data (i32.const 1024) "/painel")
  (data (i32.const 1032) "pedra_da_sorte")
  (data (i32.const 1047) "Pedra da sorte")
  (data (i32.const 1062) "Painel do plugin")
  (data (i32.const 1079) "Um exemplo de tela feita por plugin.")
  (data (i32.const 1116) "Clique num botao:")
  (data (i32.const 1134) "Fazer dia")
  (data (i32.const 1144) "Fazer noite")
  (data (i32.const 1156) "Ganhar uma pedra da sorte")
  (data (i32.const 1182) "Fechar")
  (data (i32.const 1189) "Painel: bom dia!")
  (data (i32.const 1206) "Painel: boa noite!")
  (data (i32.const 1225) "Painel: tome uma pedra da sorte (clique direito com ela abre o painel).")
  (data (i32.const 1297) "Plugin painel: use /painel")

  (func (export "buffer") (result i32) (i32.const 8192))

  ;; The lucky stone: a green pebble with a light top left, drawn into memory at 4096.
  (func (export "items")
    (local $x i32) (local $y i32) (local $p i32) (local $d i32) (local $shade i32)
    (block $rows_done
      (loop $rows
        (br_if $rows_done (i32.ge_s (local.get $y) (i32.const 16)))
        (local.set $x (i32.const 0))
        (block $cols_done
          (loop $cols
            (br_if $cols_done (i32.ge_s (local.get $x) (i32.const 16)))
            (local.set $p (i32.add (i32.const 4096) (i32.shl (i32.add (i32.mul (local.get $y) (i32.const 16)) (local.get $x)) (i32.const 2))))
            ;; Distance squared from the middle (7.5, 8.5), times 4.
            (local.set $d (i32.add
              (i32.mul (i32.sub (i32.shl (local.get $x) (i32.const 1)) (i32.const 15)) (i32.sub (i32.shl (local.get $x) (i32.const 1)) (i32.const 15)))
              (i32.mul (i32.sub (i32.shl (local.get $y) (i32.const 1)) (i32.const 17)) (i32.sub (i32.shl (local.get $y) (i32.const 1)) (i32.const 17)))))
            (if (i32.lt_s (local.get $d) (i32.const 196))
              (then
                ;; Lighter toward the top left, darker at the rim.
                (local.set $shade (i32.sub (i32.const 120) (i32.add (i32.shl (i32.add (local.get $x) (local.get $y)) (i32.const 2)) (i32.shr_s (local.get $d) (i32.const 3)))))
                (i32.store8 (local.get $p) (i32.add (i32.const 40) (i32.shr_s (local.get $shade) (i32.const 1))))
                (i32.store8 (i32.add (local.get $p) (i32.const 1)) (i32.add (i32.const 110) (local.get $shade)))
                (i32.store8 (i32.add (local.get $p) (i32.const 2)) (i32.add (i32.const 60) (i32.shr_s (local.get $shade) (i32.const 1))))
                (i32.store8 (i32.add (local.get $p) (i32.const 3)) (i32.const 255))))
            (if (i32.ge_s (local.get $d) (i32.const 196))
              (then (if (i32.lt_s (local.get $d) (i32.const 256))
                (then
                  (i32.store8 (local.get $p) (i32.const 20))
                  (i32.store8 (i32.add (local.get $p) (i32.const 1)) (i32.const 50))
                  (i32.store8 (i32.add (local.get $p) (i32.const 2)) (i32.const 28))
                  (i32.store8 (i32.add (local.get $p) (i32.const 3)) (i32.const 255))))))
            (local.set $x (i32.add (local.get $x) (i32.const 1)))
            (br $cols)))
        (local.set $y (i32.add (local.get $y) (i32.const 1)))
        (br $rows)))
    (drop (call $add_item (i32.const 1032) (i32.const 14) (i32.const 1047) (i32.const 14) (i32.const 4096) (i32.const 16) (i32.const 0))))

  (func (export "init")
    (call $add_command (i32.const 1024) (i32.const 7))
    (call $say (i32.const 1297) (i32.const 26)))

  (func $open
    (call $screen_open (i32.const 1062) (i32.const 16))
    (call $screen_text (i32.const 1079) (i32.const 36))
    (call $screen_text (i32.const 1116) (i32.const 17))
    (call $screen_button (i32.const 1) (i32.const 1134) (i32.const 9))
    (call $screen_button (i32.const 2) (i32.const 1144) (i32.const 11))
    (call $screen_button (i32.const 3) (i32.const 1156) (i32.const 25))
    (call $screen_button (i32.const 4) (i32.const 1182) (i32.const 6)))

  (func (export "command") (param $p i32) (param $n i32) (result i32)
    (call $open)
    (i32.const 1))

  (func (export "item_used") (param $p i32) (param $n i32) (result i32)
    (call $open)
    (i32.const 1))

  (func (export "button") (param $id i32)
    (if (i32.eq (local.get $id) (i32.const 1))
      (then (call $set_time (f32.const 1000)) (call $say (i32.const 1189) (i32.const 16)) (call $screen_close)))
    (if (i32.eq (local.get $id) (i32.const 2))
      (then (call $set_time (f32.const 13000)) (call $say (i32.const 1206) (i32.const 18)) (call $screen_close)))
    (if (i32.eq (local.get $id) (i32.const 3))
      (then (drop (call $give (i32.const 1032) (i32.const 14) (i32.const 1))) (call $say (i32.const 1225) (i32.const 71)) (call $screen_close)))
    (if (i32.eq (local.get $id) (i32.const 4))
      (then (call $screen_close))))
)
