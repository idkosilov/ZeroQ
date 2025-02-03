import argparse
import csv
import operator
from collections import defaultdict
from pathlib import Path

import matplotlib.pyplot as plt


def read_and_process_data(
    filename: str,
) -> dict[str, list[tuple[int, dict[str, float]]]]:
    """Read benchmark data from a CSV file and process it."""
    data = defaultdict(list)
    with Path(filename).open(encoding='utf-8') as f:
        reader = csv.DictReader(f)
        for row in reader:
            queue_type = row['param:queue_type']
            item_size = int(row['param:item_size'])
            metrics = {
                'min': float(row['min']),
                'max': float(row['max']),
                'mean': float(row['mean']),
                'median': float(row['median']),
                'stddev': float(row['stddev']),
                'ops': float(row['ops']),
            }
            data[queue_type].append((item_size, metrics))

    for queue_type in data:
        data[queue_type].sort(key=operator.itemgetter(0))

    return data


def format_size(size: int) -> str:
    """Format size in bytes into human-readable format (B, KiB, MiB, GiB)."""
    if size < 1024:
        return f'{size}B'
    if size < 1024**2:
        return f'{size / 1024:.1f}KiB'
    if size < 1024**3:
        return f'{size / (1024**2):.1f}MiB'
    return f'{size / (1024**3):.1f}GiB'


def format_time(time: float) -> str:
    """Format time in seconds into human-readable format (ns, μs, ms, s)."""
    time_ns = time * 1e9
    if time_ns < 1000:
        return f'{time_ns:.1f}ns'
    if time_ns < 1e6:
        return f'{time_ns / 1000:.1f}μs'
    if time_ns < 1e9:
        return f'{time_ns / 1e6:.1f}ms'
    return f'{time:.3f}s'


def draw_plot(
    data: dict[str, list[tuple[int, dict[str, float]]]], output_file: str
) -> None:
    """Draw a plot with dark theme and improved styling."""
    plt.style.use('dark_background')
    fig, ax = plt.subplots(figsize=(12, 8), facecolor='#1a1a1a')

    # Custom color palette
    colors = {
        'zeroq': '#00ff9d',  # Neon green
        'multiprocessing': '#ff4d4d',  # Bright red
    }

    markers = {'zeroq': 'o', 'multiprocessing': 'D'}

    # Plot configuration
    ax.set_facecolor('#1a1a1a')
    ax.grid(visible=True, color='#404040', linestyle='--', alpha=0.6)

    # Plot data
    for queue_type, values in data.items():
        sizes = [x[0] for x in values]
        metrics = {
            'mean': [x[1]['mean'] for x in values],
            'min': [x[1]['min'] for x in values],
            'max': [x[1]['max'] for x in values],
        }
        ax.loglog(
            sizes,
            metrics['mean'],
            marker=markers[queue_type],
            label=queue_type,
            linewidth=2.5,
            markersize=10,
            color=colors[queue_type],
            markerfacecolor='none',
            markeredgewidth=2,
            linestyle='-',
            alpha=0.9,
        )
        ax.fill_between(
            sizes,
            metrics['min'],
            metrics['max'],
            color=colors[queue_type],
            alpha=0.1,
        )

        # Annotations with better positioning
        for size, metric_values in values:
            x = size
            y = metric_values['mean']
            label = format_time(y)

            ax.annotate(
                label,
                xy=(x, y),
                xytext=(0, 5 if queue_type == 'zeroq' else 40),
                textcoords='offset points',
                fontsize=9,
                color=colors[queue_type],
                ha='center',
                va='bottom' if queue_type == 'zeroq' else 'top',
                rotation=45,
                alpha=0.85,
                bbox={
                    'boxstyle': 'round,pad=0.2',
                    'facecolor': '#2a2a2a',
                    'edgecolor': colors[queue_type],
                    'alpha': 0.7,
                },
            )

    # Axis styling
    ax.set_xscale('log')
    ax.set_yscale('log')
    ax.spines['bottom'].set_color('#606060')
    ax.spines['left'].set_color('#606060')
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)
    ax.set_ylim(bottom=ax.get_ylim()[0] * 0.8)

    # Labels styling
    ax.set_ylabel('Time per Operation', fontsize=13, color='white', labelpad=15)
    ax.set_xlabel('Payload Size', fontsize=13, color='white', labelpad=15)

    # Labels styling
    ax.set_ylabel('Time per Operation', fontsize=13, color='white', labelpad=15)
    ax.set_xlabel('Payload Size', fontsize=13, color='white', labelpad=15)

    # Tick parameters
    ax.tick_params(axis='both', which='both', colors='white', labelsize=11)
    ax.tick_params(axis='x', which='minor', bottom=False)

    # Custom x-ticks formatting
    all_sizes = sorted({x[0] for values in data.values() for x in values})
    ax.set_xticks(all_sizes)
    ax.set_xticklabels(
        [format_size(size) for size in all_sizes],
        fontsize=10,
        color='#cccccc',
        rotation=30,
    )

    # Y-ticks formatting
    yticks = ax.get_yticks()
    ax.set_yticklabels(
        [format_time(tick) for tick in yticks], fontsize=8, color='#cccccc'
    )

    # Legend styling
    legend = ax.legend(
        fontsize=12,
        framealpha=0.2,
        loc='upper left',
        facecolor='#2a2a2a',
        edgecolor='#404040',
    )
    for text in legend.get_texts():
        text.set_color('white')

    plt.tight_layout()

    if output_file:
        plt.savefig(
            output_file,
            dpi=300,
            bbox_inches='tight',
            facecolor='#1a1a1a',
            edgecolor='none',
        )
    plt.show()


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('input_file')
    parser.add_argument('-o', '--output')
    args = parser.parse_args()
    processed_data = read_and_process_data(args.input_file)
    draw_plot(processed_data, args.output)
