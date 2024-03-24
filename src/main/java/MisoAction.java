public enum MisoAction implements Animated
{
	UP(4,10),
	DOWN(4,10),
	LEFT(4,10),
	RIGHT(4,10),
	CURLED(2,40),
	LAYING(4,20),
	SITTING(4,20),

	LICKING(4, 40),
	RISING(2,40),
	SLEEP(1, 10);

	private final int frameCount, delay;
	private MisoAction(int frameCount, int delay)
	{
		this.frameCount = frameCount;
		this.delay = delay;
	}

	@Override
	public int getFrameCount()
	{
		return this.frameCount;
	}

	@Override
	public int getDelay()
	{
		return this.delay;
	}
}