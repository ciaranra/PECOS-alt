; Bell State QIR Program
; Creates a Bell state |00⟩ + |11⟩ and measures both qubits

; Module declaration
; ModuleID = 'bell_state'
source_filename = "bell_state.qir"

; External function declarations (QIS interface)
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__cnot__body(i64, i64)
declare void @__quantum__qis__m__body(i64, i64)
declare i64 @__quantum__rt__qubit_allocate()
declare void @__quantum__rt__qubit_release(i64)
declare i64 @__quantum__rt__result_allocate()
declare i1 @__quantum__rt__result_get_zero(i64)

; Entry point
define void @bell_state() {
entry:
  ; Allocate two qubits
  %q0 = call i64 @__quantum__rt__qubit_allocate()
  %q1 = call i64 @__quantum__rt__qubit_allocate()

  ; Allocate two result registers
  %r0 = call i64 @__quantum__rt__result_allocate()
  %r1 = call i64 @__quantum__rt__result_allocate()

  ; Create Bell state: H on q0, then CNOT from q0 to q1
  call void @__quantum__qis__h__body(i64 %q0)
  call void @__quantum__qis__cnot__body(i64 %q0, i64 %q1)

  ; Measure both qubits
  call void @__quantum__qis__m__body(i64 %q0, i64 %r0)
  call void @__quantum__qis__m__body(i64 %q1, i64 %r1)

  ; Clean up
  call void @__quantum__rt__qubit_release(i64 %q0)
  call void @__quantum__rt__qubit_release(i64 %q1)

  ret void
}

; Main entry point for testing
define i32 @main() {
  call void @bell_state()
  ret i32 0
}