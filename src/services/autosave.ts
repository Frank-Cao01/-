export type AsyncSave<T> = (value: T) => Promise<void>;

/**
 * 每条笔记使用独立串行队列，确保较早的保存结果不会覆盖较新的编辑。
 */
export class AutosaveQueue<T> {
  private timer: ReturnType<typeof setTimeout> | null = null;
  private chain: Promise<void> = Promise.resolve();
  private pending: T | null = null;
  private composing = false;

  constructor(
    private readonly save: AsyncSave<T>,
    private readonly delay = 500,
  ) {}

  schedule(value: T) {
    this.pending = value;
    if (this.composing) return;
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      this.timer = null;
      // 自动保存失败由编辑器状态展示；这里消费 Promise，避免产生未处理拒绝。
      void this.flush().catch(() => undefined);
    }, this.delay);
  }

  setComposing(value: boolean) {
    this.composing = value;
    if (!value && this.pending !== null) this.schedule(this.pending);
  }

  async flush() {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    const value = this.pending;
    this.pending = null;
    if (value === null) return this.chain;
    // 一次失败不应毒化后续串行队列，用户修正后必须仍可重试保存。
    this.chain = this.chain.catch(() => undefined).then(() => this.save(value));
    return this.chain;
  }

  cancel() {
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
    this.pending = null;
  }
}
