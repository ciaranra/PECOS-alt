#!/usr/bin/env python3
"""
Generate architecture diagrams for PECOS Engine System.

This script creates visual diagrams showing the relationships between
different components in the PECOS engine architecture.
"""

import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
from matplotlib.patches import FancyBboxPatch, FancyArrowPatch
import matplotlib.lines as mlines

def create_engine_architecture_diagram():
    """Create the main engine architecture diagram."""
    fig, ax = plt.subplots(1, 1, figsize=(14, 10))
    ax.set_xlim(0, 14)
    ax.set_ylim(0, 10)
    ax.axis('off')
    
    # Define colors
    api_color = '#E8F4FD'
    orchestrator_color = '#BBE1FA'
    hybrid_color = '#7FB5D5'
    engine_color = '#5A96C7'
    
    # User API Layer
    api_box = FancyBboxPatch((1, 8.5), 12, 1.2, 
                              boxstyle="round,pad=0.1",
                              facecolor=api_color,
                              edgecolor='black',
                              linewidth=2)
    ax.add_patch(api_box)
    ax.text(7, 9.1, 'User API Layer', ha='center', va='center', fontsize=14, fontweight='bold')
    ax.text(3, 8.7, 'qasm_sim()', ha='center', va='center', fontsize=11)
    ax.text(7, 8.7, 'llvm_sim()', ha='center', va='center', fontsize=11)
    ax.text(11, 8.7, 'guppy_sim()', ha='center', va='center', fontsize=11)
    
    # MonteCarloEngine
    mc_box = FancyBboxPatch((1, 6.8), 12, 1.4,
                            boxstyle="round,pad=0.1",
                            facecolor=orchestrator_color,
                            edgecolor='black',
                            linewidth=2)
    ax.add_patch(mc_box)
    ax.text(7, 7.7, 'MonteCarloEngine', ha='center', va='center', fontsize=14, fontweight='bold')
    ax.text(7, 7.3, '• Parallel execution', ha='center', va='center', fontsize=10)
    ax.text(7, 7.0, '• Shot distribution', ha='center', va='center', fontsize=10)
    
    # HybridEngine
    hybrid_box = FancyBboxPatch((1, 4.8), 12, 1.6,
                                boxstyle="round,pad=0.1",
                                facecolor=hybrid_color,
                                edgecolor='black',
                                linewidth=2)
    ax.add_patch(hybrid_box)
    ax.text(7, 5.9, 'HybridEngine', ha='center', va='center', fontsize=14, fontweight='bold')
    ax.text(7, 5.5, 'Combines Classical + Quantum', ha='center', va='center', fontsize=10)
    ax.text(7, 5.2, 'Manages control flow', ha='center', va='center', fontsize=10)
    
    # Classical Engines
    classical_box = FancyBboxPatch((0.5, 2), 6, 2.5,
                                   boxstyle="round,pad=0.1",
                                   facecolor=engine_color,
                                   edgecolor='black',
                                   linewidth=2)
    ax.add_patch(classical_box)
    ax.text(3.5, 4.2, 'ClassicalControlEngine', ha='center', va='center', fontsize=12, fontweight='bold')
    ax.text(3.5, 3.8, '• QasmEngine', ha='center', va='center', fontsize=10)
    ax.text(3.5, 3.5, '• LlvmEngine', ha='center', va='center', fontsize=10)
    ax.text(3.5, 3.2, '• PhirEngine', ha='center', va='center', fontsize=10)
    ax.text(3.5, 2.7, 'Generates commands', ha='center', va='center', fontsize=9, style='italic')
    ax.text(3.5, 2.4, 'Handles measurements', ha='center', va='center', fontsize=9, style='italic')
    
    # Quantum System
    quantum_box = FancyBboxPatch((7.5, 2), 6, 2.5,
                                 boxstyle="round,pad=0.1",
                                 facecolor=engine_color,
                                 edgecolor='black',
                                 linewidth=2)
    ax.add_patch(quantum_box)
    ax.text(10.5, 4.2, 'QuantumSystem', ha='center', va='center', fontsize=12, fontweight='bold')
    
    # Noise Models
    noise_box = FancyBboxPatch((8, 3.2), 2.2, 1,
                               boxstyle="round,pad=0.05",
                               facecolor='#F0F0F0',
                               edgecolor='gray',
                               linewidth=1)
    ax.add_patch(noise_box)
    ax.text(9.1, 3.9, 'NoiseModel', ha='center', va='center', fontsize=10, fontweight='bold')
    ax.text(9.1, 3.6, '• Depolarizing', ha='center', va='center', fontsize=8)
    ax.text(9.1, 3.4, '• Biased', ha='center', va='center', fontsize=8)
    
    # Quantum Engines
    qengine_box = FancyBboxPatch((10.8, 3.2), 2.2, 1,
                                 boxstyle="round,pad=0.05",
                                 facecolor='#F0F0F0',
                                 edgecolor='gray',
                                 linewidth=1)
    ax.add_patch(qengine_box)
    ax.text(11.9, 3.9, 'QuantumEngine', ha='center', va='center', fontsize=10, fontweight='bold')
    ax.text(11.9, 3.6, '• StateVec', ha='center', va='center', fontsize=8)
    ax.text(11.9, 3.4, '• SparseStab', ha='center', va='center', fontsize=8)
    
    # Draw arrows
    # API to MonteCarloEngine
    arrow1 = FancyArrowPatch((7, 8.5), (7, 8.2),
                            connectionstyle="arc3,rad=0",
                            arrowstyle='->,head_width=0.3,head_length=0.3',
                            color='black',
                            linewidth=2)
    ax.add_patch(arrow1)
    
    # MonteCarloEngine to HybridEngine
    arrow2 = FancyArrowPatch((7, 6.8), (7, 6.4),
                            connectionstyle="arc3,rad=0",
                            arrowstyle='->,head_width=0.3,head_length=0.3',
                            color='black',
                            linewidth=2)
    ax.add_patch(arrow2)
    
    # HybridEngine to Classical and Quantum
    arrow3 = FancyArrowPatch((5, 4.8), (4, 4.5),
                            connectionstyle="arc3,rad=0.2",
                            arrowstyle='->,head_width=0.3,head_length=0.3',
                            color='black',
                            linewidth=2)
    ax.add_patch(arrow3)
    
    arrow4 = FancyArrowPatch((9, 4.8), (10, 4.5),
                            connectionstyle="arc3,rad=-0.2",
                            arrowstyle='->,head_width=0.3,head_length=0.3',
                            color='black',
                            linewidth=2)
    ax.add_patch(arrow4)
    
    # Noise to Quantum Engine
    arrow5 = FancyArrowPatch((10.2, 3.7), (10.8, 3.7),
                            connectionstyle="arc3,rad=0",
                            arrowstyle='->,head_width=0.2,head_length=0.2',
                            color='gray',
                            linewidth=1)
    ax.add_patch(arrow5)
    
    # Data flow annotations
    ax.text(5.5, 4.5, 'ByteMessage', ha='center', va='center', fontsize=8, style='italic', rotation=-20)
    ax.text(8.5, 4.5, 'ByteMessage', ha='center', va='center', fontsize=8, style='italic', rotation=20)
    
    # Add title
    ax.text(7, 9.7, 'PECOS Engine System Architecture', ha='center', va='center', 
            fontsize=16, fontweight='bold')
    
    # Add legend
    legend_elements = [
        mpatches.Rectangle((0, 0), 1, 1, facecolor=api_color, edgecolor='black', label='User APIs'),
        mpatches.Rectangle((0, 0), 1, 1, facecolor=orchestrator_color, edgecolor='black', label='Orchestration'),
        mpatches.Rectangle((0, 0), 1, 1, facecolor=hybrid_color, edgecolor='black', label='Coordination'),
        mpatches.Rectangle((0, 0), 1, 1, facecolor=engine_color, edgecolor='black', label='Engines'),
    ]
    ax.legend(handles=legend_elements, loc='lower center', ncol=4, bbox_to_anchor=(0.5, -0.05))
    
    plt.tight_layout()
    plt.savefig('engine_architecture.png', dpi=300, bbox_inches='tight')
    plt.savefig('engine_architecture.pdf', bbox_inches='tight')
    print("Saved: engine_architecture.png and engine_architecture.pdf")

def create_data_flow_diagram():
    """Create the data flow diagram."""
    fig, ax = plt.subplots(1, 1, figsize=(12, 8))
    ax.set_xlim(0, 12)
    ax.set_ylim(0, 8)
    ax.axis('off')
    
    # Define positions
    positions = {
        'user': (1, 6),
        'classical': (3, 4),
        'noise': (6, 4),
        'quantum': (9, 4),
        'result': (11, 6),
    }
    
    # Draw components
    for name, (x, y) in positions.items():
        if name == 'user':
            box = FancyBboxPatch((x-0.5, y-0.3), 1, 0.6,
                                boxstyle="round,pad=0.1",
                                facecolor='#E8F4FD',
                                edgecolor='black')
            ax.add_patch(box)
            ax.text(x, y, 'User\nInput', ha='center', va='center', fontsize=10)
        elif name == 'result':
            box = FancyBboxPatch((x-0.5, y-0.3), 1, 0.6,
                                boxstyle="round,pad=0.1",
                                facecolor='#E8F4FD',
                                edgecolor='black')
            ax.add_patch(box)
            ax.text(x, y, 'Shot\nResults', ha='center', va='center', fontsize=10)
        else:
            box = FancyBboxPatch((x-0.8, y-0.5), 1.6, 1,
                                boxstyle="round,pad=0.1",
                                facecolor='#BBE1FA',
                                edgecolor='black')
            ax.add_patch(box)
            
            if name == 'classical':
                ax.text(x, y, 'Classical\nEngine', ha='center', va='center', fontsize=10, fontweight='bold')
            elif name == 'noise':
                ax.text(x, y, 'Noise\nModel', ha='center', va='center', fontsize=10, fontweight='bold')
            elif name == 'quantum':
                ax.text(x, y, 'Quantum\nEngine', ha='center', va='center', fontsize=10, fontweight='bold')
    
    # Draw data flow arrows
    flows = [
        ('user', 'classical', 'Program'),
        ('classical', 'noise', 'Commands'),
        ('noise', 'quantum', 'Noisy Ops'),
        ('quantum', 'classical', 'Measurements'),
        ('classical', 'result', 'Results'),
    ]
    
    for i, (start, end, label) in enumerate(flows):
        x1, y1 = positions[start]
        x2, y2 = positions[end]
        
        if start == 'quantum' and end == 'classical':
            # Curved arrow for return path
            arrow = FancyArrowPatch((x1-0.8, y1), (x2+0.8, y2),
                                  connectionstyle="arc3,rad=-0.3",
                                  arrowstyle='->,head_width=0.3,head_length=0.3',
                                  color='red',
                                  linewidth=2)
        else:
            # Straight arrows
            arrow = FancyArrowPatch((x1+0.5, y1), (x2-0.8, y2),
                                  connectionstyle="arc3,rad=0",
                                  arrowstyle='->,head_width=0.3,head_length=0.3',
                                  color='blue' if i < 3 else 'green',
                                  linewidth=2)
        ax.add_patch(arrow)
        
        # Add labels
        if start == 'quantum' and end == 'classical':
            ax.text((x1+x2)/2, y1-1, label, ha='center', va='center', 
                   fontsize=9, style='italic', color='red')
        else:
            mid_x = (x1 + x2) / 2
            mid_y = (y1 + y2) / 2
            ax.text(mid_x, mid_y+0.3, label, ha='center', va='center', 
                   fontsize=9, style='italic')
    
    # Add control flow box
    control_box = FancyBboxPatch((2, 2), 8, 0.8,
                                boxstyle="round,pad=0.1",
                                facecolor='#FFFACD',
                                edgecolor='orange',
                                linestyle='--')
    ax.add_patch(control_box)
    ax.text(6, 2.4, 'Control Flow: EngineStage::NeedsProcessing ↔ EngineStage::Complete',
           ha='center', va='center', fontsize=9, style='italic')
    
    # Title
    ax.text(6, 7.5, 'PECOS Engine Data Flow', ha='center', va='center',
           fontsize=16, fontweight='bold')
    
    # Legend
    blue_line = mlines.Line2D([], [], color='blue', linewidth=2, label='Command Flow')
    red_line = mlines.Line2D([], [], color='red', linewidth=2, label='Measurement Flow')
    green_line = mlines.Line2D([], [], color='green', linewidth=2, label='Result Flow')
    ax.legend(handles=[blue_line, red_line, green_line], 
             loc='lower center', ncol=3, bbox_to_anchor=(0.5, -0.05))
    
    plt.tight_layout()
    plt.savefig('engine_data_flow.png', dpi=300, bbox_inches='tight')
    plt.savefig('engine_data_flow.pdf', bbox_inches='tight')
    print("Saved: engine_data_flow.png and engine_data_flow.pdf")

if __name__ == "__main__":
    create_engine_architecture_diagram()
    create_data_flow_diagram()
    print("\nDiagrams created successfully!")