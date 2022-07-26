from threading import Thread
from other import printother

class TestClass(object):
  def __init__(self, value):
    self._var = value
    try:
      self.DoSomething()
    except ValueError:
      pass

  def DoSomething(self):
    for i in range(0, 5):
      if i < self._var:
        print('{0} is less than the value'.format(i), flush=True)
      else:
        print('{0} might be more'.format(i), flush=True)

    raise ValueError('Done')

def task():
  t = TestClass(3)

  printother()

  t._var = 99
  t.DoSomething()


def Main():
    # thread_1 = Thread(target=task)
    # thread_2 = Thread(target=task)

    # thread_1.start()
    # thread_2.start()

    # thread_1.join()
    # thread_2.join()

    task()


Main()
