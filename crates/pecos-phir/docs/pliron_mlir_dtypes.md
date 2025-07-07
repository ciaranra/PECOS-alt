# Core sata-structures in Pliron:

## Context
  - The Context is the central data structure that holds all IR-related data, such as operations, types, and attributes
  - It acts as a contrain for the IR and provides methods for creating and manipulating IR elements

## Operations
  - Operations represent individual instructions or nodes in the IR.
  - Each operation has a set of operands (inputs), results (outputs), and attributes (metadata)

## Type
  - Types represent the data types used in the IR, such as integers, floats, or custom types.
  - Users can define their own types by implementing the Type trait.

## Attribute
  - Attributes are used to attach additional information to operations or types.
  - Examples include constant values, debug informtion, or optimization hints.

## Pass
  - A Pass is a transformation or analysis that operates on the IR
  - Users can define custom passes to implement optimizations, analyses, or lowering transformations


