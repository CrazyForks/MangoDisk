import { TREEMAP_TILE_KINDS } from '@/lib/models/analysis';
import type { DirectoryEntryInfo, TreemapTile } from '@/lib/models/analysis';

interface TreemapRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

type TreemapLayoutNode =
  | {
      kind: typeof TREEMAP_TILE_KINDS.entry;
      entry: DirectoryEntryInfo;
      bytes: number;
    }
  | {
      kind: typeof TREEMAP_TILE_KINDS.remainder;
      entry: null;
      bytes: number;
      entryCount: number;
    };

export interface TreemapLayoutOptions {
  minimumVisibleShare?: number;
}

const DEFAULT_MINIMUM_VISIBLE_SHARE = 0.0075;

/** Computes percentage-based tiles without DOM, Store, or platform access. */
export class TreemapLayoutUtils {
  static layout(entries: DirectoryEntryInfo[], options: TreemapLayoutOptions = {}): TreemapTile[] {
    const candidates = entries.filter(entry => entry.bytes > 0).sort((left, right) => right.bytes - left.bytes);
    if (!candidates.length) return [];

    const total = candidates.reduce((sum, entry) => sum + entry.bytes, 0);
    const minimumVisibleShare = Math.min(1, Math.max(0, options.minimumVisibleShare ?? DEFAULT_MINIMUM_VISIBLE_SHARE));

    // Treemap area is proportional to byte share. Keeping only entries that can
    // receive a useful share avoids unreadable pixel fragments independently
    // of the current window size. Do not impose a fixed item limit: a folder
    // containing many similarly sized files still needs to show every item
    // because none of them is less meaningful than the others.
    let visibleCount = candidates.length;
    while (visibleCount > 1 && candidates[visibleCount - 1].bytes / total < minimumVisibleShare) {
      visibleCount -= 1;
    }

    const visibleEntries = candidates.slice(0, visibleCount);
    const hiddenEntries = candidates.slice(visibleCount);
    const nodes: TreemapLayoutNode[] = visibleEntries.map(entry => ({
      kind: TREEMAP_TILE_KINDS.entry,
      entry,
      bytes: entry.bytes,
    }));

    if (hiddenEntries.length) {
      nodes.push({
        kind: TREEMAP_TILE_KINDS.remainder,
        entry: null,
        bytes: hiddenEntries.reduce((sum, entry) => sum + entry.bytes, 0),
        entryCount: hiddenEntries.length,
      });
    }

    // The aggregated remainder may be larger than individual visible entries.
    // Sorting it with the other nodes preserves balanced partitions and keeps
    // its visual area truthful instead of forcing it into a narrow final strip.
    nodes.sort((left, right) => right.bytes - left.bytes);
    return TreemapLayoutUtils.partition(nodes, {
      left: 0,
      top: 0,
      width: 100,
      height: 100,
    });
  }

  private static partition(entries: TreemapLayoutNode[], rect: TreemapRect): TreemapTile[] {
    if (entries.length === 0) return [];
    if (entries.length === 1) return [{ ...entries[0], ...rect }];

    const total = entries.reduce((sum, entry) => sum + entry.bytes, 0);
    const splitIndex = TreemapLayoutUtils.findBalancedSplit(entries, total);
    const first = entries.slice(0, splitIndex);
    const second = entries.slice(splitIndex);
    const firstTotal = first.reduce((sum, entry) => sum + entry.bytes, 0);
    const ratio = total > 0 ? firstTotal / total : 0.5;

    if (rect.width >= rect.height) {
      const firstWidth = rect.width * ratio;
      return [
        ...TreemapLayoutUtils.partition(first, { ...rect, width: firstWidth }),
        ...TreemapLayoutUtils.partition(second, {
          left: rect.left + firstWidth,
          top: rect.top,
          width: rect.width - firstWidth,
          height: rect.height,
        }),
      ];
    }

    const firstHeight = rect.height * ratio;
    return [
      ...TreemapLayoutUtils.partition(first, { ...rect, height: firstHeight }),
      ...TreemapLayoutUtils.partition(second, {
        left: rect.left,
        top: rect.top + firstHeight,
        width: rect.width,
        height: rect.height - firstHeight,
      }),
    ];
  }

  private static findBalancedSplit(entries: TreemapLayoutNode[], total: number): number {
    const target = total / 2;
    let running = 0;
    let bestIndex = 1;
    let bestDistance = Number.POSITIVE_INFINITY;

    for (let index = 1; index < entries.length; index += 1) {
      running += entries[index - 1].bytes;
      const distance = Math.abs(target - running);
      if (distance < bestDistance) {
        bestDistance = distance;
        bestIndex = index;
      }
    }
    return bestIndex;
  }
}
