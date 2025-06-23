"""
QIR Debugging and Diagnostic Tools

This module provides Python utilities for debugging QIR execution issues,
detecting format problems, and providing helpful error information.
"""

from typing import Dict, List, Any, Optional
import os
from pathlib import Path

try:
    from pecos_rslib import validate_qir_format_detailed, get_qir_diagnostic_report
    _DIAGNOSTICS_AVAILABLE = True
except ImportError:
    _DIAGNOSTICS_AVAILABLE = False


class QirDiagnostics:
    """Enhanced QIR debugging and diagnostic utilities."""
    
    @staticmethod
    def is_available() -> bool:
        """Check if QIR diagnostics are available."""
        return _DIAGNOSTICS_AVAILABLE
    
    @staticmethod
    def validate_qir_file(qir_path: str) -> Dict[str, Any]:
        """
        Validate a QIR file and get detailed diagnostic information.
        
        Args:
            qir_path: Path to the QIR file to validate
            
        Returns:
            Dictionary containing validation results:
            - format_valid: bool - Whether the QIR format is valid
            - format_errors: List[str] - List of format errors found
            - runtime_warnings: List[str] - List of potential runtime issues
            - statistics: Dict - QIR file statistics
            
        Raises:
            ImportError: If diagnostics are not available
            FileNotFoundError: If QIR file doesn't exist
        """
        if not _DIAGNOSTICS_AVAILABLE:
            raise ImportError("QIR diagnostics not available - pecos_rslib not installed")
        
        if not os.path.exists(qir_path):
            raise FileNotFoundError(f"QIR file not found: {qir_path}")
            
        return validate_qir_format_detailed(qir_path)
    
    @staticmethod
    def get_execution_report() -> str:
        """
        Get the current QIR execution diagnostic report.
        
        Returns:
            String containing execution diagnostics
        """
        if not _DIAGNOSTICS_AVAILABLE:
            return "QIR diagnostics not available - pecos_rslib not installed"
        
        return get_qir_diagnostic_report()
    
    @staticmethod
    def analyze_qir_content(qir_content: str) -> Dict[str, Any]:
        """
        Analyze QIR content without needing a file.
        
        Args:
            qir_content: QIR content as string
            
        Returns:
            Dictionary with analysis results
        """
        analysis = {
            "format_type": "unknown",
            "issues": [],
            "suggestions": [],
            "statistics": {}
        }
        
        # Detect QIR format type
        has_opaque_types = "%Result = type opaque" in qir_content or "%Qubit = type opaque" in qir_content
        uses_i64_ops = "__quantum__qis__h__body(i64" in qir_content
        uses_ptr_ops = "__quantum__qis__h__body(i8*" in qir_content or "__quantum__qis__h__body(%Qubit*" in qir_content
        
        if has_opaque_types and uses_ptr_ops:
            analysis["format_type"] = "standard_qir"
        elif uses_i64_ops and not has_opaque_types:
            analysis["format_type"] = "hugr_style"
        elif uses_i64_ops and uses_ptr_ops:
            analysis["format_type"] = "mixed"
            analysis["issues"].append("Mixed calling conventions detected - both i64 and pointer types")
        
        # Check for common issues
        if "define i1 @" in qir_content and "EntryPoint" in qir_content:
            analysis["issues"].append("Entry point returns i1 instead of void")
            analysis["suggestions"].append("Entry points should return void for standard QIR")
        
        if "call i32 @__quantum__qis__m__body" in qir_content:
            analysis["issues"].append("Measurements return i32 instead of void")
            analysis["suggestions"].append("Standard QIR measurements should return void")
        
        # Calculate statistics
        lines = qir_content.split('\n')
        analysis["statistics"] = {
            "total_lines": len(lines),
            "quantum_operations": qir_content.count("__quantum__qis__"),
            "measurements": qir_content.count("__quantum__qis__m__body"),
            "has_entry_point": "EntryPoint" in qir_content,
            "has_opaque_types": has_opaque_types,
            "format_type": analysis["format_type"]
        }
        
        return analysis
    
    @staticmethod
    def diagnose_execution_error(error_message: str, qir_path: Optional[str] = None) -> Dict[str, Any]:
        """
        Diagnose a QIR execution error and provide suggestions.
        
        Args:
            error_message: The error message from QIR execution
            qir_path: Optional path to the QIR file that failed
            
        Returns:
            Dictionary with diagnosis and suggestions
        """
        diagnosis = {
            "error_type": "unknown",
            "likely_cause": "unknown",
            "suggestions": [],
            "qir_analysis": None
        }
        
        # Analyze error message
        error_lower = error_message.lower()
        
        if "index out of bounds" in error_lower:
            diagnosis["error_type"] = "index_out_of_bounds"
            diagnosis["likely_cause"] = "Qubit or result index exceeds allocated resources"
            diagnosis["suggestions"] = [
                "Check qubit allocation in your QIR code",
                "Verify qubit indices are within the allocated range",
                "Look for hardcoded large indices that might be incorrect",
                "Ensure qubits are allocated before use"
            ]
        
        elif "segmentation fault" in error_lower or "abort" in error_lower:
            diagnosis["error_type"] = "memory_error"
            diagnosis["likely_cause"] = "Memory corruption or invalid pointer access"
            diagnosis["suggestions"] = [
                "Check for QIR format incompatibilities",
                "Verify entry point function signature",
                "Look for mixed calling conventions",
                "Try using HUGR format instead of standard QIR"
            ]
        
        elif "format" in error_lower or "incompatible" in error_lower:
            diagnosis["error_type"] = "format_error"
            diagnosis["likely_cause"] = "QIR format is incompatible with runtime"
            diagnosis["suggestions"] = [
                "Use QIR format validation to check compatibility",
                "Try different LLVM convention (hugr vs qir)",
                "Check for proper opaque type declarations",
                "Verify function signatures match expected format"
            ]
        
        elif "entry point" in error_lower:
            diagnosis["error_type"] = "entry_point_error"
            diagnosis["likely_cause"] = "Entry point function not found or invalid"
            diagnosis["suggestions"] = [
                "Ensure function has EntryPoint attribute",
                "Check entry point function name",
                "Verify entry point returns void for standard QIR"
            ]
        
        # Analyze QIR file if provided
        if qir_path and os.path.exists(qir_path):
            try:
                with open(qir_path, 'r') as f:
                    qir_content = f.read()
                diagnosis["qir_analysis"] = QirDiagnostics.analyze_qir_content(qir_content)
            except Exception as e:
                diagnosis["qir_analysis"] = {"error": f"Failed to analyze QIR file: {e}"}
        
        return diagnosis
    
    @staticmethod
    def print_diagnosis(diagnosis: Dict[str, Any]) -> None:
        """Print a formatted diagnosis report."""
        print("🔍 QIR Execution Diagnosis")
        print("=" * 40)
        
        print(f"Error Type: {diagnosis['error_type']}")
        print(f"Likely Cause: {diagnosis['likely_cause']}")
        
        if diagnosis["suggestions"]:
            print("\n💡 Suggestions:")
            for i, suggestion in enumerate(diagnosis["suggestions"], 1):
                print(f"  {i}. {suggestion}")
        
        if diagnosis.get("qir_analysis"):
            analysis = diagnosis["qir_analysis"]
            print(f"\n📊 QIR Analysis:")
            print(f"  Format Type: {analysis.get('format_type', 'unknown')}")
            
            if analysis.get("issues"):
                print("  Issues Found:")
                for issue in analysis["issues"]:
                    print(f"    - {issue}")
            
            if analysis.get("statistics"):
                stats = analysis["statistics"]
                print(f"  Statistics: {stats.get('quantum_operations', 0)} quantum ops, "
                      f"{stats.get('measurements', 0)} measurements")


def validate_qir(qir_path: str, verbose: bool = True) -> bool:
    """
    Convenience function to validate a QIR file.
    
    Args:
        qir_path: Path to QIR file
        verbose: Whether to print detailed information
        
    Returns:
        True if valid, False otherwise
    """
    try:
        result = QirDiagnostics.validate_qir_file(qir_path)
        
        if verbose:
            print(f"🔍 QIR Validation: {qir_path}")
            print("-" * 40)
            
            if result["format_valid"]:
                print("✅ Format: Valid")
            else:
                print("❌ Format: Invalid")
                for error in result["format_errors"]:
                    print(f"   Error: {error}")
            
            if result["runtime_warnings"]:
                print("⚠️  Runtime Warnings:")
                for warning in result["runtime_warnings"]:
                    print(f"   Warning: {warning}")
            
            stats = result["statistics"]
            print(f"📊 Statistics:")
            print(f"   Format Type: {'Standard QIR' if stats['has_opaque_types'] else 'HUGR-style'}")
            print(f"   Quantum Operations: {stats['quantum_operations']}")
            print(f"   Has Entry Point: {stats['has_entry_point']}")
        
        return result["format_valid"] and not result["runtime_warnings"]
        
    except Exception as e:
        if verbose:
            print(f"❌ Validation Failed: {e}")
        return False


def diagnose_error(error_message: str, qir_path: Optional[str] = None) -> None:
    """
    Convenience function to diagnose a QIR execution error.
    
    Args:
        error_message: Error message from QIR execution
        qir_path: Optional path to QIR file
    """
    diagnosis = QirDiagnostics.diagnose_execution_error(error_message, qir_path)
    QirDiagnostics.print_diagnosis(diagnosis)