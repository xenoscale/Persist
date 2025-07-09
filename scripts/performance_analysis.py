#!/usr/bin/env python3
"""
Performance analysis script for Persist system.
This script analyzes benchmark results and generates performance insights.
"""

import json
import os
import sys
import statistics
import matplotlib.pyplot as plt
import pandas as pd
from pathlib import Path
import argparse

def setup_matplotlib():
    """Setup matplotlib for plotting."""
    plt.style.use('seaborn-v0_8' if 'seaborn-v0_8' in plt.style.available else 'default')
    plt.rcParams['figure.figsize'] = (12, 8)
    plt.rcParams['font.size'] = 10

def parse_criterion_results(criterion_dir):
    """Parse criterion benchmark results."""
    results = {}
    criterion_path = Path(criterion_dir)
    
    if not criterion_path.exists():
        print(f"Criterion directory not found: {criterion_dir}")
        return results
    
    for bench_group in criterion_path.iterdir():
        if bench_group.is_dir():
            group_name = bench_group.name
            estimates_file = bench_group / "base" / "estimates.json"
            
            if estimates_file.exists():
                with open(estimates_file) as f:
                    data = json.load(f)
                    
                results[group_name] = {
                    'mean_ns': data.get('mean', {}).get('point_estimate', 0),
                    'std_dev_ns': data.get('std_dev', {}).get('point_estimate', 0),
                    'throughput': data.get('throughput', {})
                }
    
    return results

def parse_hyperfine_results(hyperfine_file):
    """Parse hyperfine benchmark results."""
    if not os.path.exists(hyperfine_file):
        print(f"Hyperfine results file not found: {hyperfine_file}")
        return {}
    
    with open(hyperfine_file) as f:
        data = json.load(f)
    
    results = {}
    for result in data.get('results', []):
        command = result.get('command', 'unknown')
        results[command] = {
            'mean_s': result.get('mean', 0),
            'stddev_s': result.get('stddev', 0),
            'min_s': result.get('min', 0),
            'max_s': result.get('max', 0),
            'times': result.get('times', [])
        }
    
    return results

def analyze_memory_profile(dhat_file):
    """Analyze dhat memory profiling results."""
    if not os.path.exists(dhat_file):
        print(f"DHAT file not found: {dhat_file}")
        return {}
    
    with open(dhat_file) as f:
        data = json.load(f)
    
    # Extract key memory metrics
    return {
        'total_bytes': data.get('dhatFileVersion', {}).get('total_bytes', 0),
        'total_blocks': data.get('dhatFileVersion', {}).get('total_blocks', 0),
        'peak_bytes': data.get('dhatFileVersion', {}).get('at_t_peak_bytes', 0),
        'peak_blocks': data.get('dhatFileVersion', {}).get('at_t_peak_blocks', 0)
    }

def generate_performance_report(test_results_dir):
    """Generate comprehensive performance report."""
    results_path = Path(test_results_dir)
    
    # Parse different benchmark results
    criterion_results = parse_criterion_results(results_path / "benchmarks")
    hyperfine_results = parse_hyperfine_results(results_path / "hyperfine_results.json")
    memory_analysis = analyze_memory_profile(results_path / "profiling" / "dhat-heap.json")
    
    # Generate report
    report = []
    report.append("# Persist Performance Analysis Report")
    report.append(f"Generated: {pd.Timestamp.now()}")
    report.append("")
    
    # Criterion Results
    if criterion_results:
        report.append("## Criterion Benchmark Results")
        report.append("")
        
        for bench_name, data in criterion_results.items():
            mean_ms = data['mean_ns'] / 1_000_000
            std_dev_ms = data['std_dev_ns'] / 1_000_000
            
            report.append(f"### {bench_name}")
            report.append(f"- Mean execution time: {mean_ms:.2f} ms")
            report.append(f"- Standard deviation: {std_dev_ms:.2f} ms")
            
            if data['throughput']:
                throughput = data['throughput']
                report.append(f"- Throughput: {throughput}")
            report.append("")
    
    # Hyperfine Results
    if hyperfine_results:
        report.append("## Hyperfine Benchmark Results")
        report.append("")
        
        for command, data in hyperfine_results.items():
            mean_ms = data['mean_s'] * 1000
            stddev_ms = data['stddev_s'] * 1000
            
            report.append(f"### Command: {command}")
            report.append(f"- Mean: {mean_ms:.2f} ms")
            report.append(f"- Std Dev: {stddev_ms:.2f} ms")
            report.append(f"- Min: {data['min_s']*1000:.2f} ms")
            report.append(f"- Max: {data['max_s']*1000:.2f} ms")
            report.append("")
    
    # Memory Analysis
    if memory_analysis and any(memory_analysis.values()):
        report.append("## Memory Usage Analysis")
        report.append("")
        report.append(f"- Total bytes allocated: {memory_analysis.get('total_bytes', 0):,}")
        report.append(f"- Total blocks allocated: {memory_analysis.get('total_blocks', 0):,}")
        report.append(f"- Peak bytes: {memory_analysis.get('peak_bytes', 0):,}")
        report.append(f"- Peak blocks: {memory_analysis.get('peak_blocks', 0):,}")
        report.append("")
    
    # Performance Insights
    report.append("## Performance Insights")
    report.append("")
    
    if criterion_results:
        # Find fastest and slowest operations
        operations = [(name, data['mean_ns']) for name, data in criterion_results.items()]
        if operations:
            operations.sort(key=lambda x: x[1])
            fastest = operations[0]
            slowest = operations[-1]
            
            report.append(f"- Fastest operation: {fastest[0]} ({fastest[1]/1_000_000:.2f} ms)")
            report.append(f"- Slowest operation: {slowest[0]} ({slowest[1]/1_000_000:.2f} ms)")
            
            if len(operations) > 1:
                speedup = slowest[1] / fastest[1]
                report.append(f"- Performance ratio: {speedup:.1f}x difference")
            report.append("")
    
    # Recommendations
    report.append("## Recommendations")
    report.append("")
    
    # Analyze performance patterns and provide recommendations
    if criterion_results:
        save_times = []
        load_times = []
        compression_times = []
        
        for name, data in criterion_results.items():
            time_ms = data['mean_ns'] / 1_000_000
            if 'save' in name.lower():
                save_times.append(time_ms)
            elif 'load' in name.lower():
                load_times.append(time_ms)
            elif 'compression' in name.lower():
                compression_times.append(time_ms)
        
        if save_times:
            avg_save = statistics.mean(save_times)
            report.append(f"- Average save time: {avg_save:.2f} ms")
            if avg_save > 100:
                report.append("  - Consider optimizing save operations for better performance")
        
        if load_times:
            avg_load = statistics.mean(load_times)
            report.append(f"- Average load time: {avg_load:.2f} ms")
            if avg_load > 50:
                report.append("  - Consider optimizing load operations")
        
        if compression_times:
            avg_compression = statistics.mean(compression_times)
            report.append(f"- Average compression time: {avg_compression:.2f} ms")
    
    report.append("")
    report.append("### General Recommendations")
    report.append("- Review flame graphs to identify hot code paths")
    report.append("- Consider parallel processing for batch operations")
    report.append("- Monitor memory usage patterns for optimization opportunities")
    report.append("- Benchmark different compression algorithms for your use case")
    
    return "\n".join(report)

def create_performance_charts(test_results_dir, output_dir):
    """Create performance visualization charts."""
    setup_matplotlib()
    
    results_path = Path(test_results_dir)
    output_path = Path(output_dir)
    output_path.mkdir(exist_ok=True)
    
    criterion_results = parse_criterion_results(results_path / "benchmarks")
    hyperfine_results = parse_hyperfine_results(results_path / "hyperfine_results.json")
    
    # Chart 1: Criterion benchmark comparison
    if criterion_results:
        bench_names = list(criterion_results.keys())
        mean_times = [data['mean_ns'] / 1_000_000 for data in criterion_results.values()]
        std_devs = [data['std_dev_ns'] / 1_000_000 for data in criterion_results.values()]
        
        plt.figure(figsize=(12, 6))
        bars = plt.bar(bench_names, mean_times, yerr=std_devs, capsize=5)
        plt.title('Criterion Benchmark Results')
        plt.ylabel('Time (ms)')
        plt.xticks(rotation=45, ha='right')
        plt.tight_layout()
        
        # Color bars based on performance
        for i, bar in enumerate(bars):
            if mean_times[i] < 10:
                bar.set_color('green')
            elif mean_times[i] < 50:
                bar.set_color('yellow')
            else:
                bar.set_color('red')
        
        plt.savefig(output_path / 'criterion_benchmarks.png', dpi=300, bbox_inches='tight')
        plt.close()
    
    # Chart 2: Performance over data size (if available)
    size_data = {}
    for name, data in criterion_results.items():
        if any(size in name.lower() for size in ['1kb', '10kb', '100kb', '1000kb']):
            # Extract size from name
            for size in ['1KB', '10KB', '100KB', '1000KB']:
                if size.lower() in name.lower():
                    size_data[size] = data['mean_ns'] / 1_000_000
                    break
    
    if len(size_data) > 1:
        sizes = list(size_data.keys())
        times = list(size_data.values())
        
        plt.figure(figsize=(10, 6))
        plt.plot(sizes, times, 'o-', linewidth=2, markersize=8)
        plt.title('Performance vs Data Size')
        plt.xlabel('Data Size')
        plt.ylabel('Time (ms)')
        plt.grid(True, alpha=0.3)
        plt.tight_layout()
        plt.savefig(output_path / 'performance_vs_size.png', dpi=300, bbox_inches='tight')
        plt.close()
    
    print(f"Performance charts saved to {output_path}")

def main():
    parser = argparse.ArgumentParser(description='Analyze Persist performance results')
    parser.add_argument('--results-dir', default='test_results',
                        help='Directory containing test results')
    parser.add_argument('--output-dir', default='test_results/analysis',
                        help='Directory to save analysis output')
    parser.add_argument('--charts', action='store_true',
                        help='Generate performance charts')
    
    args = parser.parse_args()
    
    if not os.path.exists(args.results_dir):
        print(f"Results directory not found: {args.results_dir}")
        sys.exit(1)
    
    # Generate performance report
    print("Generating performance report...")
    report = generate_performance_report(args.results_dir)
    
    # Save report
    os.makedirs(args.output_dir, exist_ok=True)
    report_file = os.path.join(args.output_dir, 'performance_report.md')
    with open(report_file, 'w') as f:
        f.write(report)
    
    print(f"Performance report saved to: {report_file}")
    
    # Generate charts if requested
    if args.charts:
        try:
            create_performance_charts(args.results_dir, args.output_dir)
        except ImportError:
            print("matplotlib not available - skipping chart generation")
        except Exception as e:
            print(f"Error generating charts: {e}")
    
    # Print summary
    print("\nPerformance Analysis Summary:")
    print("=" * 50)
    
    criterion_results = parse_criterion_results(os.path.join(args.results_dir, "benchmarks"))
    if criterion_results:
        print(f"Criterion benchmarks analyzed: {len(criterion_results)}")
        
        times = [data['mean_ns'] / 1_000_000 for data in criterion_results.values()]
        if times:
            print(f"Average execution time: {statistics.mean(times):.2f} ms")
            print(f"Fastest operation: {min(times):.2f} ms")
            print(f"Slowest operation: {max(times):.2f} ms")
    
    hyperfine_results = parse_hyperfine_results(os.path.join(args.results_dir, "hyperfine_results.json"))
    if hyperfine_results:
        print(f"Hyperfine benchmarks analyzed: {len(hyperfine_results)}")
    
    memory_analysis = analyze_memory_profile(os.path.join(args.results_dir, "profiling", "dhat-heap.json"))
    if memory_analysis and memory_analysis.get('total_bytes', 0) > 0:
        print(f"Peak memory usage: {memory_analysis['peak_bytes']:,} bytes")

if __name__ == "__main__":
    main()
