from pecos.slr import Main, SlrConverter, Block, Repeat
from pecos.qeclib.steane.steane_class import Steane

# Create the test program
prog = Main(
    c := Steane("c"),
    c.px(),
)

# Print the full SLR structure
def print_slr(obj, indent=0):
    prefix = "  " * indent
    obj_type = type(obj).__name__
    
    # Print the object type
    print(f"{prefix}{obj_type}", end="")
    
    # Print key attributes
    if hasattr(obj, 'name'):
        print(f" name={obj.name}", end="")
    if hasattr(obj, 'cond'):
        print(f" cond={obj.cond}", end="")
    if hasattr(obj, 'n'):
        print(f" n={obj.n}", end="")
    print()
    
    # Print child operations
    if hasattr(obj, 'ops') and obj.ops:
        for op in obj.ops:
            print_slr(op, indent + 1)
    elif hasattr(obj, 'blocks') and obj.blocks:
        for block in obj.blocks:
            print_slr(block, indent + 1)
    elif hasattr(obj, 'body') and obj.body:
        for item in obj.body:
            print_slr(item, indent + 1)

print_slr(prog)
