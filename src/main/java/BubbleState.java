public enum BubbleState implements Animated
{
	ZZZ(30, 4),
	HEART(50, 4),
	NONE(-1, -1);

	private int delay, frames;
	BubbleState(int delay, int frames)
	{
		this.delay = delay;
		this.frames = frames;
	}
	@Override
	public int getFrameCount()
	{
		return frames;
	}

	@Override
	public int getDelay()
	{
		return delay;
	}
}
