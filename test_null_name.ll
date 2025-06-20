%Result = type opaque
%Qubit = type opaque

declare void @__quantum__qis__h__body(%Qubit*)
declare void @__quantum__qis__m__body(%Qubit*, %Result*)
declare void @__quantum__rt__result_record_output(%Result*, i8*)

define void @main() #0 {
    ; Apply Hadamard
    call void @__quantum__qis__h__body(%Qubit* null)

    ; Measure qubit
    call void @__quantum__qis__m__body(%Qubit* null, %Result* inttoptr (i64 0 to %Result*))

    ; Record result with NULL name - this might trigger the segfault
    call void @__quantum__rt__result_record_output(%Result* inttoptr (i64 0 to %Result*), i8* null)

    ret void
}

attributes #0 = { "EntryPoint" }