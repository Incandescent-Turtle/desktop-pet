import java.awt.KeyboardFocusManager;
import java.awt.event.KeyAdapter;
import java.awt.event.KeyEvent;
import java.util.HashMap;
import java.util.Map;

public class KeyboardHandler extends KeyAdapter
{
	private final Map<Integer, Boolean> pressedKeys = new HashMap<>();

	private Main main;

	public KeyboardHandler(Main main)
	{
		this.main = main;
	}

	{
		KeyboardFocusManager.getCurrentKeyboardFocusManager().addKeyEventDispatcher(event -> {
			synchronized(KeyboardHandler.class)
			{
				if(event.getID() == KeyEvent.KEY_PRESSED)
				{
					pressedKeys.put(event.getKeyCode(), true);
					if(event.getKeyCode() == KeyEvent.VK_ESCAPE)
					{
						System.exit(0);
					}
				} else if(event.getID() == KeyEvent.KEY_RELEASED)
				{
					pressedKeys.put(event.getKeyCode(), false);
				}
				return false;
			}
		});
	}

	public boolean isDown(int keyCode)
	{
		return pressedKeys.getOrDefault(keyCode, false);
	}
}