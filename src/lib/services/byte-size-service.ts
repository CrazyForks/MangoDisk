import { OperatingSystemService } from '@/lib/services/operating-system-service';
import { BYTE_UNIT_BASES, FormatUtils, type ByteUnitBase } from '@/lib/utils/format';

/**
 * Owns the unit convention for every user-facing byte value.
 *
 * Finder and macOS storage surfaces use decimal units. Windows Explorer uses
 * a 1024 base while retaining the KB/MB/GB labels. Centralizing that difference
 * keeps application sizes, volume capacity, cleanup results, progress, history,
 * and displayed thresholds consistent within the current operating system.
 *
 * This service is presentation-only. Inputs remain raw bytes, so platform display
 * conventions cannot alter scans, ordering, threshold behavior, persisted evidence,
 * or cleanup execution.
 */
export class ByteSizeService {
  static bytes(bytes: number): string {
    return FormatUtils.bytes(bytes, this.unitBase());
  }

  private static unitBase(): ByteUnitBase {
    return OperatingSystemService.isMacOs() ? BYTE_UNIT_BASES.decimal : BYTE_UNIT_BASES.binary;
  }
}
