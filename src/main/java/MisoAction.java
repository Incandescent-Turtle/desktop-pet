public enum MisoAction
{
	UP(4,10),
	DOWN(4,10),
	LEFT(4,10),
	RIGHT(4,10),
	CURLED(2,40),
	LAYING(4,20),
	SITTING(8,40),
	RISING(2,40);

	private final int frameCount, delay;
	private MisoAction(int frameCount, int delay)
	{
		this.frameCount = frameCount;
		this.delay = delay;
	}

	protected int getFrameCount()
	{
		return this.frameCount;
	}

	protected int getDelay()
	{
		return this.delay;
	}
}