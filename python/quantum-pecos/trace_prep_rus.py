import sys
from pecos.slr import Main, SlrConverter
from pecos.qeclib.steane.steane_class import Steane

# Create the test program
prog = Main(
    c := Steane("c"),
    c.px(),
)

# Convert to Guppy and analyze
converter = SlrConverter(prog)

# Hook into the builder to trace function generation
from pecos.slr.gen_codes.guppy.ir_builder import IRBuilder
from pecos.slr.gen_codes.guppy.ir import Comment

original_convert = IRBuilder._convert_condition

def traced_convert(self, cond):
    # Print what condition we're converting
    print(f"Converting condition: {cond}")
    if hasattr(cond, 'sym'):
        print(f"  - Symbol: {cond.sym}")
    if hasattr(cond, 'index'):
        print(f"  - Index: {cond.index}")
    if hasattr(cond, 'left') and hasattr(cond, 'right'):
        print(f"  - Comparison: {cond.left} == {cond.right}")
    
    # Check context
    if hasattr(self, 'current_function_name'):
        print(f"  - In function: {self.current_function_name}")
    if hasattr(self, 'scope_manager') and self.scope_manager.is_in_loop():
        print(f"  - Inside loop: True")
    
    return original_convert(self, cond)

IRBuilder._convert_condition = traced_convert

# Generate the code
guppy_code = converter.guppy()

# Find the prep_rus function
lines = guppy_code.split('\n')
for i, line in enumerate(lines):
    if 'def steane_prep_rus' in line:
        print("\nGenerated prep_rus function:")
        for j in range(i, min(i+15, len(lines))):
            print(lines[j])
        break
