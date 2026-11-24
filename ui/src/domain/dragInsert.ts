/** Where a dragged item lands in a vertical list. */

/**
 * The insertion index for a pointer at `y`, given each card's vertical
 * midpoint in order: before the first card whose midpoint is below the pointer.
 */
export function insertionIndex(y: number, midpoints: readonly number[]): number {
  const index = midpoints.findIndex((mid) => y < mid);
  return index === -1 ? midpoints.length : index;
}

/**
 * The index a card moved from `from` ends up at when dropped at insertion
 * index `at` (which counts the card itself), or null when it wouldn't move.
 */
export function moveTarget(from: number, at: number): number | null {
  const to = at > from ? at - 1 : at;
  return to === from ? null : to;
}
