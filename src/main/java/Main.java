import javax.imageio.ImageIO;
import javax.swing.JFrame;
import javax.swing.JPanel;
import javax.swing.SwingUtilities;
import javax.swing.Timer;
import java.awt.*;
import java.awt.event.KeyEvent;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.geom.AffineTransform;
import java.awt.image.BufferedImage;
import java.io.IOException;
import java.util.*;
import java.util.List;

public class Main extends JPanel
{
	private final JFrame f;
	private KeyboardHandler keyboardHandler;
	private final Map<String, List<BufferedImage>> frames;
	private List<BufferedImage> currFrames;

	private int frameNum = 0;
	private MisoAction action ;
	private volatile int animationSteps = 0;
	private Direction layingDir = Direction.RIGHT;

	private enum Direction { RIGHT, LEFT}

	private State state = State.DEFAULT;
	private Point wanderLoc = new Point(0,0);
	private enum State { DEFAULT, WANDER }
	private BubbleState bubbleState = BubbleState.NONE;
	private final Map<String, List<BufferedImage>> bubbleFrames;
	private List<BufferedImage> currBubbleFrames;
	private int bubbleFrameNum = 0;
	private int bubbleSteps = 0;



	public Main()
	{
		frames = loadSprites(MisoAction.values());
		bubbleFrames = loadSprites(BubbleState.values());
		f = new JFrame();
		f.setType(javax.swing.JFrame.Type.UTILITY);
		f.setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
		f.setUndecorated(true);
		var dim = new Dimension(100, 100);
		f.setPreferredSize(dim);
		f.setMinimumSize(dim);
		f.setLocationRelativeTo(null);
		f.setLocation(0,0);
		f.setAlwaysOnTop(true);
		setOpaque(false);
		keyboardHandler = new KeyboardHandler(this);
		f.addMouseMotionListener(new MouseAdapter()
		{
			@Override
			public void mouseDragged(MouseEvent e)
			{
				SwingUtilities.invokeLater(() -> {
					f.setLocation(e.getLocationOnScreen().x-f.getWidth()/2,e.getLocationOnScreen().y-f.getHeight()/2);
					var c = changeAction(MisoAction.RISING);
					if(c)
					{
						frameNum=0;
					}
				});
			}
		});
		f.addMouseListener(new MouseAdapter()
		{
			@Override
			public void mouseReleased(MouseEvent e)
			{
				super.mouseReleased(e);
				SwingUtilities.invokeLater(()->{
					if(action == MisoAction.RISING)
					{
						changeAction(MisoAction.LAYING);
						frameNum=1;
					}
				});
			}

			@Override
			public void mouseClicked(MouseEvent e)
			{
				super.mouseClicked(e);
				bubbleState = BubbleState.HEART;
				bubbleFrameNum = 0;
			}
		});
		f.add(this);
		f.setBackground(new Color(1.0f, 1.0f, 1.0f, 0.0f));
		f.setVisible(true);

		changeAction(MisoAction.CURLED);

		Timer mainTimer = new Timer(10, (e) -> {
			updateAction();
			doAction();
			updateAnimation();
			manageBubble();
			if(keyboardHandler.isDown(KeyEvent.VK_W))
			{
				tryWander(true);
			}
			f.repaint();
		});

		Timer wanderTimer = new Timer(1000*60*10, (e) -> tryWander(false));
		mainTimer.start();
		wanderTimer.start();
	}

	private void manageBubble()
	{
		if(bubbleState != BubbleState.HEART)
		{
			if(action == MisoAction.SLEEP || action == MisoAction.CURLED)
			{
				bubbleState = BubbleState.ZZZ;
			} else if(action != MisoAction.LICKING && action != MisoAction.SITTING){
				bubbleState = BubbleState.NONE;
			}
		}
		bubbleSteps++;
		currBubbleFrames = bubbleFrames.getOrDefault(bubbleState.name(), bubbleFrames.get(BubbleState.HEART.name()));
		if(bubbleSteps >= bubbleState.getDelay())
		{
			bubbleFrameNum++;
			bubbleSteps = 0;
		}
		if(bubbleFrameNum >= bubbleState.getFrameCount())
		{
			bubbleFrameNum = 0;
			if(bubbleState == BubbleState.HEART)
			{
				bubbleState = BubbleState.NONE;
			}
		}
	}

	private void tryWander(boolean force)
	{
		var rand = new Random();
		// 50% of time
		if(!force && rand.nextBoolean())
		{
			return;
		}
		Toolkit toolkit = Toolkit.getDefaultToolkit();
		Dimension screenSize = toolkit.getScreenSize();

		state = State.WANDER;
		var screenLoc = f.getLocationOnScreen();
		Point newLoc;
		do {
			newLoc = new Point(rand.nextInt(screenSize.width-f.getWidth()-20)+10, rand.nextInt(screenSize.height-f.getHeight()-20)+10);
		} while(xAndYWithinThreshold(screenLoc, newLoc, 400));
		wanderLoc = newLoc;
	}

	private void updateAnimation()
	{
		animationSteps++;
		if(animationSteps >= action.getDelay())
		{
			if(action == MisoAction.LAYING && frameNum == action.getFrameCount()-1)
			{
				if((animationSteps - action.getDelay()) > 40)
				{
					var num = new Random().nextBoolean();
					animationSteps = 0;
					frameNum = 0;
					if(num)
					{
						changeAction(MisoAction.CURLED);
					} else {
						changeAction(MisoAction.SLEEP);
					}
				}
			} else if(action == MisoAction.SITTING && frameNum == action.getFrameCount()-1) {
				changeAction(MisoAction.LICKING);
				animationSteps = 0;
				frameNum = 0;
			} else {
				frameNum++;
				animationSteps =0;
			}
		}
		if(frameNum >= action.getFrameCount())
		{
			frameNum = 0;
		}
	}

	private void doAction()
	{
		var loc = f.getLocation();
		switch(action)
		{
			case RIGHT -> loc.translate(1,0);
			case LEFT -> loc.translate(-1,0);
			case UP -> loc.translate(0,-1);
			case DOWN -> loc.translate(0,1);
		}
		Toolkit toolkit = Toolkit.getDefaultToolkit();
		Dimension screenSize = toolkit.getScreenSize();
		if(loc.x > screenSize.width-f.getWidth() )
		{
			loc.setLocation(screenSize.width-f.getWidth(), loc.y);
		}
		if(loc.x < -10)
		{
			loc.setLocation(-10, loc.y);
		}
		if(loc.y > screenSize.height-f.getHeight())
		{
			loc.setLocation(loc.x, screenSize.height-f.getHeight());
		}
		if(loc.y < -35)
		{
			loc.setLocation(loc.x, -35);
		}
		f.setLocation(loc);
	}

	private void updateAction()
	{
		if(action != MisoAction.RISING)
		{
			if(state == State.WANDER && !isMoveKeyPressed())
			{
				var curPos = f.getLocationOnScreen();

				if(Math.abs(curPos.x-wanderLoc.x) >= 3)
				{
					if(curPos.x > wanderLoc.x)
					{
						changeAction(MisoAction.LEFT);
					} else {
						changeAction(MisoAction.RIGHT);
					}
				} else {
					if(curPos.y > wanderLoc.y)
					{
						changeAction(MisoAction.UP);
					} else {
						changeAction(MisoAction.DOWN);
					}
				}
				// if done wander (close enough to point)
				if(wanderLoc.distance(curPos) < 3)
				{
					state = State.DEFAULT;
				}
			}
			boolean changed = false;
			if(keyboardHandler.isDown(KeyEvent.VK_UP))
				changed = changeAction(MisoAction.UP);
			else if(keyboardHandler.isDown(KeyEvent.VK_DOWN))
				changed = changeAction(MisoAction.DOWN);
			else if(keyboardHandler.isDown(KeyEvent.VK_LEFT))
				changed = changeAction(MisoAction.LEFT);
			else if(keyboardHandler.isDown(KeyEvent.VK_RIGHT))
				changed = changeAction(MisoAction.RIGHT);
			else if(action != MisoAction.CURLED && action != MisoAction.SITTING && action != MisoAction.LICKING && action != MisoAction.RISING && action != MisoAction.SLEEP && action != MisoAction.LAYING)
			{
				if(action == MisoAction.LEFT)
				{
					layingDir = Direction.LEFT;
				} else if(action == MisoAction.RIGHT) {
					layingDir = Direction.RIGHT;
				}
				if(state != State.WANDER)
				{
					var num = new Random().nextInt(3);
					if(num >= 1)
					{
						changed = changeAction(MisoAction.LAYING);
					} else {
						changed = changeAction(MisoAction.SITTING);
					}
				}
			}
			if(changed)
			{
				frameNum = 0;
			}
		}
	}

	// returns if it was changed
	private boolean changeAction(MisoAction newAction)
	{
		if(newAction != action)
		{
			action=newAction;
			currFrames = frames.get(action.name());
			return true;
		}
		return false;
	}
	@Override
	protected void paintComponent(Graphics g)
	{
		super.paintComponent(g);
		if(currFrames != null)
		{
			var currImg = currFrames.get(frameNum);
			if(currImg != null)
			{
				if((action == MisoAction.LAYING || action == MisoAction.RISING || action == MisoAction.SLEEP) && layingDir == Direction.LEFT || action == MisoAction.CURLED && layingDir == Direction.RIGHT)
					currImg = flipImage(currImg);
				g.drawImage(currImg, 0,0,100,100,null);
			}
		}
		if(currBubbleFrames != null && bubbleState != BubbleState.NONE)
		{
			var currImg = currBubbleFrames.get(bubbleFrameNum);
			if(currImg != null)
			{
				int x = 30, y = 40;
				switch(action)
				{
					case SLEEP:
					case LAYING:
					case LEFT:
					case RIGHT:
						if(layingDir == Direction.LEFT)
						{
							x-=30;
						} else {
							x+=30;
						}
						break;
					case UP:
					case LICKING:
					case SITTING:
						y-=25;
				}
				g.drawImage(currImg, x, y, 30, 30, null);
			}
		}

	}

	private <T extends Enum<T> & Animated> Map<String, List<BufferedImage>> loadSprites(T[] vals) {
		var map = new HashMap<String, List<BufferedImage>>();

		for(final var action : vals)
		{
			if(action.getFrameCount() <= 0)
			{
				continue;
			}
			var list = new ArrayList<BufferedImage>();
			map.put(action.name(), list);
			final var folderName = action.name().toLowerCase();
			try
			{
				for(int i = 1; i <= action.getFrameCount(); i++)
				{
					list.add(ImageIO.read(getClass().getResource(folderName + "/" + folderName + "_" + i + ".png")));
				}
			} catch(IOException e)
			{
				throw new RuntimeException(e);
			}
		}
		return map;
	}

	public static void main(String[] args)
	{
		new Main();
	}

	// gpt generated
	private BufferedImage flipImage(BufferedImage image) {
		// Create a new BufferedImage with the same dimensions as the original image
		BufferedImage mirrored = new BufferedImage(image.getWidth(), image.getHeight(), BufferedImage.TYPE_INT_ARGB);

		// Get the graphics context of the new image
		Graphics2D g2d = mirrored.createGraphics();

		// Flip the image horizontally using AffineTransform
		AffineTransform at = new AffineTransform();
		at.concatenate(AffineTransform.getScaleInstance(-1, 1));
		at.concatenate(AffineTransform.getTranslateInstance(-image.getWidth(), 0));
		g2d.transform(at);

		// Draw the original image onto the new image
		g2d.drawImage(image, 0, 0, null);

		// Dispose of the graphics context
		g2d.dispose();

		return mirrored;
	}

	// whether the differences on the x and y axes are within or equal to the threshold
	private boolean xAndYWithinThreshold(Point p1, Point p2, int threshold)
	{
		var xDiff = Math.abs(p1.x-p2.x);
		var yDiff = Math.abs(p1.y-p2.y);
		return yDiff <= threshold && xDiff <= threshold;
	}

	private boolean isMoveKeyPressed()
	{
		return keyboardHandler.isDown(KeyEvent.VK_UP) || keyboardHandler.isDown(KeyEvent.VK_DOWN) || keyboardHandler.isDown(KeyEvent.VK_RIGHT) || keyboardHandler.isDown(KeyEvent.VK_LEFT);
	}
}
