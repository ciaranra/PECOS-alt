; ModuleID = 'quantum_module'

declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__x__body(i64)
declare void @__quantum__qis__y__body(i64)
declare void @__quantum__qis__z__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare void @__quantum__qis__cz__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)

source_filename = "quantum_module"

define i1 @main() #0 {
alloca_block:
  %"0" = alloca i1, align 1
  %"4_0" = alloca i1, align 1
  %"01" = alloca i1, align 1
  %"13_0" = alloca {}, align 8
  %"12_0" = alloca i1, align 1
  %"11_0" = alloca {}, align 8
  %"9_0" = alloca i16, align 2
  %"10_0" = alloca i16, align 2
  br label %entry_block

entry_block:                                      ; preds = %alloca_block
  br label %0

0:                                                ; preds = %entry_block
  store {} undef, {}* %"13_0", align 1
  store {} undef, {}* %"11_0", align 1
  store i16 0, i16* %"9_0", align 2
  %"9_02" = load i16, i16* %"9_0", align 2
  %qubit_i64 = zext i16 %"9_02" to i64
  call void @__quantum__qis__h__body(i64 %qubit_i64)
  store i16 %"9_02", i16* %"10_0", align 2
  %"10_03" = load i16, i16* %"10_0", align 2
  %qubit_i644 = zext i16 %"10_03" to i64
  %measurement_result = call i32 @__quantum__qis__m__body(i64 %qubit_i644, i64 0)
  %is_one = icmp ne i32 %measurement_result, 0
  store i1 %is_one, i1* %"12_0", align 1
  %"13_05" = load {}, {}* %"13_0", align 1
  %"12_06" = load i1, i1* %"12_0", align 1
  store {} %"13_05", {}* %"13_0", align 1
  store i1 %"12_06", i1* %"12_0", align 1
  %"13_07" = load {}, {}* %"13_0", align 1
  %"12_08" = load i1, i1* %"12_0", align 1
  switch i1 false, label %1 [
  ]

1:                                                ; preds = %0
  store i1 %"12_08", i1* %"01", align 1
  br label %2

2:                                                ; preds = %1
  %"09" = load i1, i1* %"01", align 1
  store i1 %"09", i1* %"4_0", align 1
  %"4_010" = load i1, i1* %"4_0", align 1
  store i1 %"4_010", i1* %"0", align 1
  %"011" = load i1, i1* %"0", align 1
  ret i1 %"011"
}

declare void @__quantum__qis__h__body(i64)

declare i32 @__quantum__qis__m__body(i64, i64)

attributes #0 = { "EntryPoint" }
