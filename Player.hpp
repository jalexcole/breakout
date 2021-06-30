#pragma once
#include "raylib.h"
#include "Entity.hpp"


class Player: public Entity {
  public:
    Vector2 acceleration;

    Player(Vector2 vector, int x, int y);

    void init();
    void update();
    void checkInput();
    void preventLeft();
    void preventRight();

  private:
    void updateVelocity();
    void updateAcceleration();
};